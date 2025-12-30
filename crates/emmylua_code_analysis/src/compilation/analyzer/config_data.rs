use emmylua_parser::{LuaAstNode, LuaChunk, LuaTableExpr};

use crate::{
    ConfigTableIndexKeys, ConfigTablePkOccurrence, LuaSemanticDeclId, LuaType, LuaTypeCache,
    compilation::analyzer::AnalysisPipeline, db_index::DbIndex, find_members_with_key, infer_expr,
    profile::Profile,
};

use super::{AnalyzeContext, infer_cache_manager::InferCacheManager};

pub struct LuaConfigDataIndexPipeline;

impl AnalysisPipeline for LuaConfigDataIndexPipeline {
    fn analyze(db: &mut DbIndex, context: &mut AnalyzeContext) {
        let _p = Profile::cond_new("luaconfig data index", context.tree_list.len() > 1);

        let mut infer_manager = InferCacheManager::new();
        let tree_list = context.tree_list.clone();
        for in_filed_tree in tree_list.iter() {
            let file_id = in_filed_tree.file_id;
            let root = in_filed_tree.value.clone();
            index_file(db, &mut infer_manager, file_id, root);
        }
    }
}

fn index_file(
    db: &mut DbIndex,
    infer_manager: &mut InferCacheManager,
    file_id: crate::FileId,
    root: LuaChunk,
) {
    let Some(decl_tree) = db.get_decl_index().get_decl_tree(&file_id) else {
        return;
    };

    let mut occurrences: Vec<ConfigTablePkOccurrence> = Vec::new();
    let infer_cache = infer_manager.get_infer_cache(file_id);

    for (decl_id, decl) in decl_tree.get_decls().iter() {
        let Some(type_cache) = db.get_type_index().get_type_cache(&(*decl_id).into()) else {
            continue;
        };

        let LuaTypeCache::DocType(LuaType::Ref(config_table_id)) = type_cache else {
            continue;
        };

        let Some(index_keys) = db
            .get_config_index()
            .get_config_table_keys(config_table_id)
            .cloned()
        else {
            continue;
        };

        let Some(expr_id) = decl.get_value_syntax_id() else {
            continue;
        };

        let Some(table_node) = expr_id.to_node_from_root(root.syntax()) else {
            continue;
        };
        let Some(table_expr) = LuaTableExpr::cast(table_node) else {
            continue;
        };

        collect_table_occurrences(
            db,
            infer_cache,
            config_table_id.clone(),
            &index_keys,
            &table_expr,
            &mut occurrences,
        );
    }

    db.get_config_index_mut()
        .add_config_table_pk_occurrences(file_id, occurrences);
}

fn collect_table_occurrences(
    db: &DbIndex,
    infer_cache: &mut crate::LuaInferCache,
    config_table: crate::LuaTypeDeclId,
    index_keys: &ConfigTableIndexKeys,
    table: &LuaTableExpr,
    out: &mut Vec<ConfigTablePkOccurrence>,
) {
    let keys = index_keys.keys();
    if keys.is_empty() {
        return;
    }

    let fields = table.get_fields().collect::<Vec<_>>();
    for field in fields {
        let Some(row_expr) = field.get_value_expr() else {
            continue;
        };

        let Ok(row_typ) = infer_expr(db, infer_cache, row_expr) else {
            continue;
        };

        match index_keys {
            ConfigTableIndexKeys::Solo(_) => {
                for key in keys {
                    let Some(member_infos) =
                        find_members_with_key(db, &row_typ, key.clone(), false)
                    else {
                        continue;
                    };

                    let Some(member_info) = member_infos.first() else {
                        continue;
                    };

                    let range = match member_info.property_owner_id {
                        Some(LuaSemanticDeclId::Member(member_id)) => {
                            member_id.get_syntax_id().get_range()
                        }
                        _ => continue,
                    };

                    out.push(ConfigTablePkOccurrence::Solo {
                        config_table: config_table.clone(),
                        key: key.clone(),
                        value: member_info.typ.clone(),
                        range,
                    });
                }
            }
            ConfigTableIndexKeys::Union(_) => {
                let mut values: Vec<LuaType> = Vec::with_capacity(keys.len());
                let mut ranges = Vec::with_capacity(keys.len());

                let mut ok = true;
                for key in keys {
                    let Some(member_infos) =
                        find_members_with_key(db, &row_typ, key.clone(), false)
                    else {
                        ok = false;
                        break;
                    };

                    let Some(member_info) = member_infos.first() else {
                        ok = false;
                        break;
                    };

                    let range = match member_info.property_owner_id {
                        Some(LuaSemanticDeclId::Member(member_id)) => {
                            member_id.get_syntax_id().get_range()
                        }
                        _ => {
                            ok = false;
                            break;
                        }
                    };

                    values.push(member_info.typ.clone());
                    ranges.push(range);
                }

                if !ok {
                    continue;
                }

                out.push(ConfigTablePkOccurrence::Union {
                    config_table: config_table.clone(),
                    values,
                    ranges,
                });
            }
        }
    }
}
