use std::collections::{HashMap, HashSet};

use emmylua_parser::{LuaAstNode, LuaExpr, LuaTableExpr};
use rowan::TextRange;

use crate::{
    DbIndex, DiagnosticCode, LuaMemberKey, LuaType, LuaTypeDeclId, RenderLevel, SemanticModel,
    attributes::VIndexAttribute,
    diagnostic::checker::{Checker, DiagnosticContext},
    humanize_type, infer_expr, infer_table_should_be,
    semantic::shared::luaconfig::BEAN,
};

pub struct DuplicateIndexValueChecker;

impl Checker for DuplicateIndexValueChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::DuplicateIndexValue];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let db = semantic_model.get_db();
        let file_id = semantic_model.get_file_id();
        let root = semantic_model.get_root().clone();

        let mut infer_cache = semantic_model.get_cache().borrow_mut();

        for table_expr in root.descendants::<LuaTableExpr>() {
            let Ok(table_should_be) =
                infer_table_should_be(db, &mut infer_cache, table_expr.clone())
            else {
                continue;
            };

            let Some(rule) = resolve_expected_container_rule(db, file_id, &table_should_be) else {
                continue;
            };

            validate_container_table_data(context, db, &mut infer_cache, &rule, &table_expr);
        }
    }
}

#[derive(Debug, Clone)]
struct ContainerIndexRule {
    fields: Vec<String>,
}

fn resolve_expected_container_rule(
    db: &DbIndex,
    file_id: crate::FileId,
    ty: &LuaType,
) -> Option<ContainerIndexRule> {
    let ty = ty.strip_attributed();
    match ty {
        LuaType::Generic(generic) => {
            let base_name = generic.get_base_type_id_ref().get_name();
            let params = generic.get_params();

            let element_ty = match base_name {
                "array" | "list" | "set" => params.first()?,
                // v.index 明确不支持 map
                _ => return None,
            };

            let fields = resolve_vindex_fields_from_element_type(db, file_id, element_ty)?;
            Some(ContainerIndexRule { fields })
        }
        LuaType::Array(array) => {
            // array<T> 在解析后可能退化为 T[]，此处按 array 语义处理。
            let element_ty = array.get_base();
            let fields = resolve_vindex_fields_from_element_type(db, file_id, element_ty)?;
            Some(ContainerIndexRule { fields })
        }
        LuaType::Union(union) => {
            let mut found: Option<ContainerIndexRule> = None;
            for inner in union.into_vec().iter() {
                let Some(rule) = resolve_expected_container_rule(db, file_id, inner) else {
                    continue;
                };

                if found.is_some() {
                    return None;
                }
                found = Some(rule);
            }
            found
        }
        LuaType::MultiLineUnion(multi) => {
            let union = multi.to_union();
            resolve_expected_container_rule(db, file_id, &union)
        }
        _ => None,
    }
}

fn resolve_vindex_fields_from_element_type(
    db: &DbIndex,
    _file_id: crate::FileId,
    ty: &LuaType,
) -> Option<Vec<String>> {
    let LuaType::Attributed(attributed) = ty else {
        return None;
    };

    let mut fields = Vec::new();
    let mut seen = HashSet::new();
    for vindex_attr in VIndexAttribute::find_all_in_uses(attributed.get_attributes().as_ref()) {
        let Some(field) = vindex_attr.get_key() else {
            continue;
        };

        if seen.insert(field.to_string()) {
            fields.push(field.to_string());
        }
    }

    if fields.is_empty() {
        return None;
    }

    resolve_expected_bean_id(db, attributed.get_base())?;
    Some(fields)
}

fn resolve_expected_bean_id(db: &DbIndex, ty: &LuaType) -> Option<LuaTypeDeclId> {
    let ty = ty.strip_attributed();
    match ty {
        LuaType::Ref(type_decl_id) | LuaType::Def(type_decl_id) => {
            if BEAN.is_bean(db, type_decl_id) {
                Some(type_decl_id.clone())
            } else {
                None
            }
        }
        LuaType::Generic(generic) => {
            let base_type_id = generic.get_base_type_id();
            if BEAN.is_bean(db, &base_type_id) {
                Some(base_type_id)
            } else {
                None
            }
        }
        LuaType::Union(union) => {
            let mut bean_ids: HashSet<LuaTypeDeclId> = HashSet::new();
            for inner in union.into_vec().iter() {
                if let Some(bean_id) = resolve_expected_bean_id(db, inner) {
                    bean_ids.insert(bean_id);
                }
            }

            if bean_ids.len() == 1 {
                bean_ids.into_iter().next()
            } else {
                None
            }
        }
        LuaType::MultiLineUnion(multi) => {
            let union = multi.to_union();
            resolve_expected_bean_id(db, &union)
        }
        _ => None,
    }
}

fn validate_container_table_data(
    context: &mut DiagnosticContext,
    db: &DbIndex,
    infer_cache: &mut crate::LuaInferCache,
    rule: &ContainerIndexRule,
    table: &LuaTableExpr,
) {
    for field_name in &rule.fields {
        let mut occurrences: HashMap<LuaType, Vec<TextRange>> = HashMap::new();

        for field in table.get_fields() {
            if !field.is_value_field() {
                continue;
            }

            let Some(value_expr) = field.get_value_expr() else {
                continue;
            };
            let LuaExpr::TableExpr(element_table) = value_expr else {
                continue;
            };

            let Some((value_typ, range)) =
                extract_literal_bean_field_value(db, infer_cache, &element_table, field_name)
            else {
                continue;
            };

            occurrences.entry(value_typ).or_default().push(range);
        }

        for (value_typ, ranges) in occurrences {
            if ranges.len() <= 1 {
                continue;
            }

            let value = humanize_type(db, &value_typ, RenderLevel::Simple);
            for range in ranges {
                context.add_diagnostic(
                    DiagnosticCode::DuplicateIndexValue,
                    range,
                    t!(
                        "Duplicate v.index field `%{field}` value `%{value}`",
                        field = field_name,
                        value = value
                    )
                    .to_string(),
                    None,
                );
            }
        }
    }
}

fn extract_literal_bean_field_value(
    db: &DbIndex,
    infer_cache: &mut crate::LuaInferCache,
    table: &LuaTableExpr,
    field_name: &str,
) -> Option<(LuaType, TextRange)> {
    for field in table.get_fields() {
        let Some(field_key) = field.get_field_key() else {
            continue;
        };

        let Ok(member_key) = LuaMemberKey::from_index_key(db, infer_cache, &field_key) else {
            continue;
        };

        let LuaMemberKey::Name(name) = member_key else {
            continue;
        };

        if name.as_str() != field_name {
            continue;
        }

        let Some(value_expr) = field.get_value_expr() else {
            continue;
        };

        let Ok(value_typ) = infer_expr(db, infer_cache, value_expr.clone()) else {
            continue;
        };

        if !is_checkable_literal_key(&value_typ) {
            return None;
        }

        return Some((value_typ, field.get_range()));
    }

    None
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
