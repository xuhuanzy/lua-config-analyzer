use std::collections::HashMap;

use emmylua_parser::{LuaAst, LuaAstNode, LuaIndexKey, LuaTableExpr};
use rowan::TextRange;

use crate::{
    DiagnosticCode, LuaMemberKey, LuaMemberOwner, LuaSemanticDeclId, LuaType, LuaTypeDeclId,
    SemanticDeclLevel, SemanticModel,
    diagnostic::checker::{Checker, DiagnosticContext, humanize_lint_type},
    find_index_operations,
};

/* 检查主键是否重复 */

pub struct DuplicatePrimaryKeyChecker;

impl Checker for DuplicatePrimaryKeyChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::DuplicatePrimaryKey];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let root = semantic_model.get_root().clone();
        if let Some(table) = root.descendants::<LuaTableExpr>().next() {
            check_duplicate_primary_key(context, semantic_model, table);
        }
    }
}

/**
 * 获取配置表的主键
 */
pub fn get_config_table_keys(
    semantic_model: &SemanticModel,
    table: &LuaTableExpr,
) -> Option<Vec<LuaMemberKey>> {
    let table_type = semantic_model.infer_table_should_be(table.clone())?;
    match table_type {
        LuaType::Ref(base) => {
            if !semantic_model.is_sub_type_of(&base, &LuaTypeDeclId::new("ConfigTable")) {
                return None;
            }
            let members =
                find_index_operations(semantic_model.get_db(), &LuaType::Ref(base.clone()))?;
            let members = members
                .iter()
                .filter(|member| matches!(member.key, LuaMemberKey::ExprType(LuaType::Integer)))
                .collect::<Vec<_>>();
            let member = members.first()?;
            // 确定成员类型为 Bean
            if let LuaType::Ref(base) = &member.typ {
                if !semantic_model.is_sub_type_of(base, &LuaTypeDeclId::new("Bean")) {
                    return None;
                }
                let mut members = semantic_model
                    .get_db()
                    .get_member_index()
                    .get_members(&LuaMemberOwner::Type(base.clone()))?
                    .to_vec();
                // 根据 member_id 的位置排序, 确保顺序稳定
                members.sort_by_key(|m| m.get_sort_key());
                let default_index = members.first()?.get_key();
                return Some(vec![default_index.clone()]);
            }
        }
        _ => return None,
    }

    None
}

fn check_duplicate_primary_key(
    context: &mut DiagnosticContext,
    semantic_model: &SemanticModel,
    table: LuaTableExpr,
) -> Option<()> {
    let db = semantic_model.get_db();
    dbg!(&table);
    let keys = get_config_table_keys(semantic_model, &table)?;
    dbg!(&keys);

    let fields = table.get_fields().collect::<Vec<_>>();

    let mut index_map: HashMap<LuaType, Vec<TextRange>> = HashMap::new();

    // 我们假设索引字段必须为 string / 整数
    for field in fields {
        // 此时 field 应该是一张表
        let typ = semantic_model
            .infer_expr(field.get_value_expr().clone()?)
            .ok()?;
        dbg!(&typ);
        let member_infos = semantic_model.get_member_infos(&typ)?;
        for member_info in member_infos {
            for key in keys.iter() {
                if member_info.key == *key {
                    let range = if let Some(LuaSemanticDeclId::Member(member_id)) =
                        member_info.property_owner_id
                    {
                        member_id.get_syntax_id().get_range()
                    } else {
                        continue;
                    };

                    index_map
                        .entry(member_info.typ.clone())
                        .or_default()
                        .push(range);
                }
            }
        }
    }
    dbg!(&index_map);
    for (name, ranges) in index_map {
        if ranges.len() > 1 {
            for range in ranges {
                context.add_diagnostic(
                    DiagnosticCode::DuplicatePrimaryKey,
                    range,
                    t!(
                        "Duplicate primary key `%{name}`.",
                        name = humanize_lint_type(db, &name)
                    )
                    .to_string(),
                    None,
                );
            }
        }
    }

    Some(())
}
