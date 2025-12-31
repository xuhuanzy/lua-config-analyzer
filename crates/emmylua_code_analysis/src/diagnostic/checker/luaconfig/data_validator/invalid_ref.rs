use std::collections::{HashMap, HashSet};

use emmylua_parser::{LuaAstNode, LuaTableExpr};
use rowan::TextRange;

use crate::{
    ConfigTablePkOccurrence, DiagnosticCode, LuaMember, LuaMemberKey, LuaMemberOwner,
    LuaSemanticDeclId, LuaType, LuaTypeCache, LuaTypeDeclId, RenderLevel, SemanticModel,
    attributes::{ConfigTableMode, VRefAttribute},
    diagnostic::checker::{Checker, DiagnosticContext},
    find_index_operations, find_members_with_key, humanize_type, infer_expr, is_sub_type_of,
    semantic::shared::luaconfig::{BEAN, CONFIG_TABLE},
};

pub struct InvalidRefChecker;

impl Checker for InvalidRefChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::InvalidRef];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let file_id = semantic_model.get_file_id();
        let db = semantic_model.get_db();
        let root = semantic_model.get_root().clone();

        let Some(decl_tree) = db.get_decl_index().get_decl_tree(&file_id) else {
            return;
        };

        // Bean -> v.ref rules (只生成签名合法的规则)
        let mut bean_rules_cache: HashMap<LuaTypeDeclId, Vec<ValidatedVRefRule>> = HashMap::new();

        // 当前文件内需要检查的 ConfigTable 数据: (table_expr, bean_id)
        let mut tables_to_check: Vec<(LuaTableExpr, LuaTypeDeclId)> = Vec::new();

        // 仅收集本文件实际用到的 (target_table -> target_keys)，用于过滤主键值集合构建
        let mut needed: HashMap<LuaTypeDeclId, HashSet<LuaMemberKey>> = HashMap::new();

        for (decl_id, decl) in decl_tree.get_decls().iter() {
            let Some(type_cache) = db.get_type_index().get_type_cache(&(*decl_id).into()) else {
                continue;
            };

            let LuaTypeCache::DocType(LuaType::Ref(config_table_id)) = type_cache else {
                continue;
            };

            if !is_sub_type_of(db, config_table_id, &CONFIG_TABLE.get_id()) {
                continue;
            }

            let Some(bean_id) = get_config_table_bean_id(db, config_table_id) else {
                continue;
            };

            let rules = bean_rules_cache
                .entry(bean_id.clone())
                .or_insert_with(|| collect_vref_rules_for_bean(context, db, &bean_id));
            if rules.is_empty() {
                continue;
            }

            let Some(expr_id) = decl.get_value_syntax_id() else {
                continue;
            };

            let Some(table_node) = expr_id.to_node_from_root(root.syntax()) else {
                continue;
            };
            let Some(table_expr) = LuaTableExpr::cast(table_node) else {
                continue;
            };

            for rule in rules.iter() {
                needed
                    .entry(rule.target_table.clone())
                    .or_default()
                    .insert(rule.target_key.clone());
            }

            tables_to_check.push((table_expr, bean_id));
        }

        if tables_to_check.is_empty() {
            return;
        }

        let pk_sets = PkValueSets::new_filtered(db, &needed);

        // 对于不存在任何主键值的 (table,key)，只报一次并跳过值校验，避免大量噪音。
        for rules in bean_rules_cache.values_mut() {
            rules.retain(|rule| {
                if pk_sets.has_any(&rule.target_table, &rule.target_key) {
                    return true;
                }

                context.add_diagnostic(
                    DiagnosticCode::InvalidRef,
                    rule.decl_range,
                    t!(
                        "Invalid v.ref: `%{table}.%{key}` has no indexed values",
                        table = rule.target_table.get_name(),
                        key = rule.target_key.to_path()
                    )
                    .to_string(),
                    None,
                );
                false
            });
        }

        let mut infer_cache = semantic_model.get_cache().borrow_mut();
        for (table_expr, bean_id) in tables_to_check {
            let Some(rules) = bean_rules_cache.get(&bean_id) else {
                continue;
            };
            if rules.is_empty() {
                continue;
            }

            validate_table_data(context, db, &mut infer_cache, &pk_sets, rules, &table_expr);
        }
    }
}

#[derive(Debug, Clone)]
struct ValidatedVRefRule {
    decl_range: TextRange,
    source_key: LuaMemberKey,
    target_table: LuaTypeDeclId,
    target_key: LuaMemberKey,
}

#[derive(Default)]
struct PkValueSets {
    // table -> (key -> set(values))
    values: HashMap<LuaTypeDeclId, HashMap<LuaMemberKey, HashSet<LuaType>>>,
}

impl PkValueSets {
    fn new_filtered(
        db: &crate::DbIndex,
        needed: &HashMap<LuaTypeDeclId, HashSet<LuaMemberKey>>,
    ) -> Self {
        let mut out = Self::default();

        for occ in db.get_config_index().iter_config_table_pk_occurrences() {
            match occ {
                ConfigTablePkOccurrence::Solo {
                    config_table,
                    key,
                    value,
                    ..
                } => {
                    if needed
                        .get(config_table)
                        .is_some_and(|keys| keys.contains(key))
                    {
                        out.insert_value(config_table, key, value.clone());
                    }
                }
                ConfigTablePkOccurrence::Union {
                    config_table,
                    keys,
                    values,
                    ..
                } => {
                    let Some(needed_keys) = needed.get(config_table) else {
                        continue;
                    };

                    for (key, value) in keys.iter().zip(values.iter()) {
                        if needed_keys.contains(key) {
                            out.insert_value(config_table, key, value.clone());
                        }
                    }
                }
            }
        }

        out
    }

    fn insert_value(&mut self, table: &LuaTypeDeclId, key: &LuaMemberKey, value: LuaType) {
        self.values
            .entry(table.clone())
            .or_default()
            .entry(key.clone())
            .or_default()
            .insert(value);
    }

    fn has_any(&self, table: &LuaTypeDeclId, key: &LuaMemberKey) -> bool {
        self.values
            .get(table)
            .and_then(|m| m.get(key))
            .is_some_and(|set| !set.is_empty())
    }

    fn contains(&self, table: &LuaTypeDeclId, key: &LuaMemberKey, value: &LuaType) -> bool {
        self.values
            .get(table)
            .and_then(|m| m.get(key))
            .is_some_and(|set| set.contains(value))
    }
}

fn validate_table_data(
    context: &mut DiagnosticContext,
    db: &crate::DbIndex,
    infer_cache: &mut crate::LuaInferCache,
    pk_sets: &PkValueSets,
    rules: &[ValidatedVRefRule],
    table: &LuaTableExpr,
) {
    for field in table.get_fields() {
        let Some(row_expr) = field.get_value_expr() else {
            continue;
        };

        let Ok(row_typ) = infer_expr(db, infer_cache, row_expr) else {
            continue;
        };

        for rule in rules {
            let Some(member_infos) =
                find_members_with_key(db, &row_typ, rule.source_key.clone(), false)
            else {
                continue;
            };

            let Some(member_info) = member_infos.first() else {
                continue;
            };

            let range = match member_info.property_owner_id {
                Some(LuaSemanticDeclId::Member(member_id)) => member_id.get_syntax_id().get_range(),
                _ => continue,
            };

            if !is_checkable_literal_key(&member_info.typ) {
                continue;
            }

            if pk_sets.contains(&rule.target_table, &rule.target_key, &member_info.typ) {
                continue;
            }

            let value = humanize_type(db, &member_info.typ, RenderLevel::Simple);
            let key_path = rule.target_key.to_path();
            let table_name = rule.target_table.get_name();

            context.add_diagnostic(
                DiagnosticCode::InvalidRef,
                range,
                t!(
                    "Invalid reference `%{value}`: not found in `%{table}.%{key}`",
                    value = value,
                    table = table_name,
                    key = key_path
                )
                .to_string(),
                None,
            );
        }
    }
}

fn collect_vref_rules_for_bean(
    context: &mut DiagnosticContext,
    db: &crate::DbIndex,
    bean_id: &LuaTypeDeclId,
) -> Vec<ValidatedVRefRule> {
    let mut out = Vec::new();

    let Some(bean_members) = db
        .get_member_index()
        .get_members(&LuaMemberOwner::Type(bean_id.clone()))
    else {
        return out;
    };

    for member in bean_members {
        let LuaMemberKey::Name(name) = member.get_key() else {
            continue;
        };

        let owner_id = LuaSemanticDeclId::Member(member.get_id());
        let Some(property) = db.get_property_index().get_property(&owner_id) else {
            continue;
        };

        let Some(vref_attr) = VRefAttribute::find_in(property) else {
            continue;
        };

        let Some(table_name) = vref_attr.get_table_name() else {
            continue;
        };
        let field_name = vref_attr.get_field_name();

        let Some((target_table, target_key)) =
            validate_vref_signature(context, db, member, table_name, field_name)
        else {
            continue;
        };

        out.push(ValidatedVRefRule {
            decl_range: member.get_range(),
            source_key: LuaMemberKey::Name(name.clone()),
            target_table,
            target_key,
        });
    }

    out
}

fn validate_vref_signature(
    context: &mut DiagnosticContext,
    db: &crate::DbIndex,
    source_member: &LuaMember,
    target_table_name: &str,
    target_field_name: Option<&str>,
) -> Option<(LuaTypeDeclId, LuaMemberKey)> {
    let range = source_member.get_range();

    // 解析到真实的 ConfigTable 类型（支持当前文件 namespace/using）
    let Some(target_decl) = db
        .get_type_index()
        .find_type_decl(source_member.get_file_id(), target_table_name)
    else {
        context.add_diagnostic(
            DiagnosticCode::InvalidRef,
            range,
            t!(
                "Invalid v.ref: unknown config table `%{table}`",
                table = target_table_name
            )
            .to_string(),
            None,
        );
        return None;
    };

    let target_table_id = target_decl.get_id();
    if !is_sub_type_of(db, &target_table_id, CONFIG_TABLE.get_id()) {
        context.add_diagnostic(
            DiagnosticCode::InvalidRef,
            range,
            t!(
                "Invalid v.ref: `%{table}` is not a ConfigTable",
                table = target_table_name
            )
            .to_string(),
            None,
        );
        return None;
    }

    let mode = db
        .get_config_index()
        .get_config_table_mode(&target_table_id);
    // 暂不处理 singleton
    if mode == ConfigTableMode::Singleton {
        return None;
    }

    let Some(index_keys) = db
        .get_config_index()
        .get_config_table_keys(&target_table_id)
    else {
        context.add_diagnostic(
            DiagnosticCode::InvalidRef,
            range,
            t!(
                "Invalid v.ref: `%{table}` has no primary keys",
                table = target_table_id.get_name()
            )
            .to_string(),
            None,
        );
        return None;
    };

    let keys = index_keys.keys();
    match mode {
        ConfigTableMode::Map => {
            if keys.len() != 1 {
                context.add_diagnostic(
                    DiagnosticCode::InvalidRef,
                    range,
                    t!(
                        "Invalid v.ref: map table `%{table}` must have exactly one primary key",
                        table = target_table_id.get_name()
                    )
                    .to_string(),
                    None,
                );
                return None;
            }

            let pk = keys[0].clone();
            if let Some(field_name) = target_field_name {
                let Some(pk_name) = pk.get_name() else {
                    context.add_diagnostic(
                        DiagnosticCode::InvalidRef,
                        range,
                        t!(
                            "Invalid v.ref: map table `%{table}` has non-name primary key",
                            table = target_table_id.get_name()
                        )
                        .to_string(),
                        None,
                    );
                    return None;
                };
                if pk_name != field_name {
                    context.add_diagnostic(
                        DiagnosticCode::InvalidRef,
                        range,
                        t!(
                            "Invalid v.ref: map table `%{table}` primary key is `%{pk}`",
                            table = target_table_id.get_name(),
                            pk = pk_name
                        )
                        .to_string(),
                        None,
                    );
                    return None;
                }
            }

            Some((target_table_id, pk))
        }
        ConfigTableMode::List => {
            let Some(field_name) = target_field_name else {
                context.add_diagnostic(
                    DiagnosticCode::InvalidRef,
                    range,
                    t!(
                        "Invalid v.ref: list table `%{table}` requires explicit `field`",
                        table = target_table_id.get_name()
                    )
                    .to_string(),
                    None,
                );
                return None;
            };

            let field_key = LuaMemberKey::Name(field_name.to_string().into());
            if !keys.iter().any(|k| k == &field_key) {
                context.add_diagnostic(
                    DiagnosticCode::InvalidRef,
                    range,
                    t!(
                        "Invalid v.ref: `%{field}` is not a primary key of `%{table}`",
                        field = field_name,
                        table = target_table_id.get_name()
                    )
                    .to_string(),
                    None,
                );
                return None;
            }

            Some((target_table_id, field_key))
        }
        ConfigTableMode::Singleton => None,
    }
}

fn get_config_table_bean_id(
    db: &crate::DbIndex,
    config_table_id: &LuaTypeDeclId,
) -> Option<LuaTypeDeclId> {
    let config_table_type = LuaType::Ref(config_table_id.clone());
    let members = find_index_operations(db, &config_table_type)?;
    let int_member = members
        .iter()
        .find(|m| matches!(m.key, LuaMemberKey::ExprType(LuaType::Integer)))?;

    let LuaType::Ref(bean_id) = &int_member.typ else {
        return None;
    };

    if !is_sub_type_of(db, bean_id, BEAN.get_id()) {
        return None;
    }

    Some(bean_id.clone())
}

fn is_checkable_literal_key(ty: &LuaType) -> bool {
    matches!(
        ty,
        LuaType::IntegerConst(_)
            | LuaType::DocIntegerConst(_)
            | LuaType::StringConst(_)
            | LuaType::DocStringConst(_)
            | LuaType::BooleanConst(_)
            | LuaType::DocBooleanConst(_)
            | LuaType::FloatConst(_)
    )
}
