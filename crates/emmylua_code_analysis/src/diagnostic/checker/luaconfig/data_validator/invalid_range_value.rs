use std::collections::HashMap;

use emmylua_parser::{LuaAstNode, LuaTableExpr};

use crate::{
    DbIndex, DiagnosticCode, LuaMemberKey, LuaMemberOwner, LuaType, LuaTypeDeclId, RenderLevel,
    SemanticModel,
    attributes::{RangeSpec, VRangeAttribute},
    db_index::LuaSemanticDeclId,
    diagnostic::checker::{Checker, DiagnosticContext},
    humanize_type, infer_expr, infer_table_should_be,
    semantic::shared::luaconfig::BEAN,
};

pub struct InvalidRangeValueChecker;

impl Checker for InvalidRangeValueChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::InvalidRangeValue];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let db = semantic_model.get_db();
        let root = semantic_model.get_root().clone();

        let mut infer_cache = semantic_model.get_cache().borrow_mut();

        let mut bean_rules_cache: HashMap<LuaTypeDeclId, HashMap<String, RangeSpec>> =
            HashMap::new();

        for table_expr in root.descendants::<LuaTableExpr>() {
            let Ok(table_should_be) =
                infer_table_should_be(db, &mut infer_cache, table_expr.clone())
            else {
                continue;
            };

            if let Some(bean_id) = resolve_expected_bean_id(db, &table_should_be) {
                let rules = bean_rules_cache
                    .entry(bean_id.clone())
                    .or_insert_with(|| collect_bean_range_rules(db, &bean_id));
                if rules.is_empty() {
                    continue;
                }

                validate_bean_table_data(context, db, &mut infer_cache, rules, &table_expr);
                continue;
            }

            if let Some(rule) = resolve_expected_container_rule(db, &table_should_be) {
                validate_container_table_data(context, db, &mut infer_cache, &rule, &table_expr);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ContainerKind {
    ArrayLike,
    Map,
}

#[derive(Debug, Clone)]
struct ContainerRangeRule {
    kind: ContainerKind,
    key: Option<RangeSpec>,
    value: Option<RangeSpec>,
}

fn resolve_expected_container_rule(_db: &DbIndex, ty: &LuaType) -> Option<ContainerRangeRule> {
    let ty = ty.strip_attributed();
    match ty {
        LuaType::Generic(generic) => {
            let base_name = generic.get_base_type_id_ref().get_name();
            let params = generic.get_params();

            match base_name {
                "array" | "list" | "set" => {
                    let element_ty = params.first()?;
                    let value = extract_range_spec_from_type(element_ty)?;
                    Some(ContainerRangeRule {
                        kind: ContainerKind::ArrayLike,
                        key: None,
                        value: Some(value),
                    })
                }
                "map" => {
                    let key_ty = params.first()?;
                    let value_ty = params.get(1)?;
                    let key = extract_range_spec_from_type(key_ty);
                    let value = extract_range_spec_from_type(value_ty);
                    if key.is_none() && value.is_none() {
                        return None;
                    }
                    Some(ContainerRangeRule {
                        kind: ContainerKind::Map,
                        key,
                        value,
                    })
                }
                _ => None,
            }
        }
        LuaType::Array(array) => {
            let element_ty = array.get_base();
            let value = extract_range_spec_from_type(element_ty)?;
            Some(ContainerRangeRule {
                kind: ContainerKind::ArrayLike,
                key: None,
                value: Some(value),
            })
        }
        LuaType::Union(union) => {
            let mut found: Option<ContainerRangeRule> = None;
            for inner in union.into_vec().iter() {
                let Some(rule) = resolve_expected_container_rule(_db, inner) else {
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
            resolve_expected_container_rule(_db, &union)
        }
        _ => None,
    }
}

fn validate_container_table_data(
    context: &mut DiagnosticContext,
    db: &DbIndex,
    infer_cache: &mut crate::LuaInferCache,
    rule: &ContainerRangeRule,
    table: &LuaTableExpr,
) {
    match rule.kind {
        ContainerKind::ArrayLike => {
            let Some(value_spec) = rule.value.as_ref() else {
                return;
            };

            for field in table.get_fields() {
                let Some(value_expr) = field.get_value_expr() else {
                    continue;
                };

                let Ok(value_typ) = infer_expr(db, infer_cache, value_expr.clone()) else {
                    continue;
                };

                let Some(value) = extract_number_value(&value_typ) else {
                    continue;
                };

                if !value_spec.contains(value) {
                    context.add_diagnostic(
                        DiagnosticCode::InvalidRangeValue,
                        value_expr.get_range(),
                        t!(
                            "Value `%{value}` is out of range `%{range}`",
                            value = humanize_type(db, &value_typ, RenderLevel::Simple),
                            range = value_spec.to_string()
                        )
                        .to_string(),
                        None,
                    );
                }
            }
        }
        ContainerKind::Map => {
            for field in table.get_fields() {
                if let Some(key_spec) = rule.key.as_ref()
                    && let Some(field_key) = field.get_field_key()
                {
                    let Ok(member_key) = LuaMemberKey::from_index_key(db, infer_cache, &field_key)
                    else {
                        continue;
                    };

                    let key_value = match &member_key {
                        LuaMemberKey::Integer(i) => Some(*i as f64),
                        LuaMemberKey::ExprType(ty) => extract_number_value(ty),
                        _ => None,
                    };

                    if let Some(key_value) = key_value
                        && !key_spec.contains(key_value)
                    {
                        context.add_diagnostic(
                            DiagnosticCode::InvalidRangeValue,
                            field.get_range(),
                            t!(
                                "Map key `%{key}` is out of range `%{range}`",
                                key = member_key.to_path(),
                                range = key_spec.to_string()
                            )
                            .to_string(),
                            None,
                        );
                    }
                }

                if let Some(value_spec) = rule.value.as_ref() {
                    let Some(value_expr) = field.get_value_expr() else {
                        continue;
                    };

                    let Ok(value_typ) = infer_expr(db, infer_cache, value_expr.clone()) else {
                        continue;
                    };

                    let Some(value) = extract_number_value(&value_typ) else {
                        continue;
                    };

                    if !value_spec.contains(value) {
                        context.add_diagnostic(
                            DiagnosticCode::InvalidRangeValue,
                            value_expr.get_range(),
                            t!(
                                "Map value `%{value}` is out of range `%{range}`",
                                value = humanize_type(db, &value_typ, RenderLevel::Simple),
                                range = value_spec.to_string()
                            )
                            .to_string(),
                            None,
                        );
                    }
                }
            }
        }
    }
}

fn validate_bean_table_data(
    context: &mut DiagnosticContext,
    db: &DbIndex,
    infer_cache: &mut crate::LuaInferCache,
    rules: &HashMap<String, RangeSpec>,
    table: &LuaTableExpr,
) {
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

        let Some(spec) = rules.get(name.as_str()) else {
            continue;
        };

        let Some(value_expr) = field.get_value_expr() else {
            continue;
        };

        let Ok(value_typ) = infer_expr(db, infer_cache, value_expr.clone()) else {
            continue;
        };

        let Some(value) = extract_number_value(&value_typ) else {
            continue;
        };

        if !spec.contains(value) {
            context.add_diagnostic(
                DiagnosticCode::InvalidRangeValue,
                value_expr.get_range(),
                t!(
                    "v.range field `%{field}` value `%{value}` is out of range `%{range}`",
                    field = name.as_str(),
                    value = humanize_type(db, &value_typ, RenderLevel::Simple),
                    range = spec.to_string()
                )
                .to_string(),
                None,
            );
        }
    }
}

fn collect_bean_range_rules(db: &DbIndex, bean_id: &LuaTypeDeclId) -> HashMap<String, RangeSpec> {
    let mut out: HashMap<String, RangeSpec> = HashMap::new();

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

        let mut spec = db
            .get_type_index()
            .get_type_cache(&member.get_id().into())
            .map(|type_cache| extract_range_spec_from_type(type_cache.as_type()))
            .unwrap_or(None);

        if spec.is_none() {
            let owner_id = LuaSemanticDeclId::Member(member.get_id());
            if let Some(property) = db.get_property_index().get_property(&owner_id)
                && let Some(attr) = VRangeAttribute::find_in(property)
                && let Ok(parsed) = attr.parse()
            {
                spec = Some(parsed);
            }
        }

        let Some(spec) = spec else {
            continue;
        };

        out.insert(name.to_string(), spec);
    }

    out
}

fn extract_range_spec_from_type(ty: &LuaType) -> Option<RangeSpec> {
    match ty {
        LuaType::Attributed(attributed) => {
            let mut found: Option<RangeSpec> = None;
            for attr in VRangeAttribute::find_all_in_uses(attributed.get_attributes().as_ref()) {
                let Ok(spec) = attr.parse() else {
                    continue;
                };

                if found.is_some() {
                    return None;
                }
                found = Some(spec);
            }

            if found.is_some() {
                found
            } else {
                extract_range_spec_from_type(attributed.get_base())
            }
        }
        LuaType::Union(union) => {
            let mut found: Option<RangeSpec> = None;
            for inner in union.into_vec().iter() {
                let Some(spec) = extract_range_spec_from_type(inner) else {
                    continue;
                };
                if let Some(existing) = found.as_ref() {
                    if existing != &spec {
                        return None;
                    }
                } else {
                    found = Some(spec);
                }
            }
            found
        }
        LuaType::MultiLineUnion(multi) => {
            let union = multi.to_union();
            extract_range_spec_from_type(&union)
        }
        _ => None,
    }
}

fn extract_number_value(ty: &LuaType) -> Option<f64> {
    match ty {
        LuaType::IntegerConst(i) | LuaType::DocIntegerConst(i) => Some(*i as f64),
        LuaType::FloatConst(f) => Some(*f),
        LuaType::Union(union) => {
            let mut found: Option<f64> = None;
            for inner in union.into_vec().iter() {
                let Some(v) = extract_number_value(inner) else {
                    continue;
                };
                if found.is_some() {
                    return None;
                }
                found = Some(v);
            }
            found
        }
        _ => None,
    }
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
            let mut bean_ids: Vec<LuaTypeDeclId> = Vec::new();
            for inner in union.into_vec().iter() {
                if let Some(bean_id) = resolve_expected_bean_id(db, inner) {
                    if !bean_ids.contains(&bean_id) {
                        bean_ids.push(bean_id);
                    }
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
