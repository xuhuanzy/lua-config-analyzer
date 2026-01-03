use emmylua_parser::{LuaAstNode, LuaTableExpr};

use crate::{
    DbIndex, DiagnosticCode, LuaType,
    attributes::{SizeSpec, VSizeAttribute},
    diagnostic::checker::{Checker, DiagnosticContext},
    infer_table_should_be,
};

pub struct InvalidSizeValueChecker;

impl Checker for InvalidSizeValueChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::InvalidSizeValue];

    fn check(context: &mut DiagnosticContext, semantic_model: &crate::SemanticModel) {
        let db = semantic_model.get_db();
        let root = semantic_model.get_root().clone();

        let mut infer_cache = semantic_model.get_cache().borrow_mut();

        for table_expr in root.descendants::<LuaTableExpr>() {
            let Ok(table_should_be) =
                infer_table_should_be(db, &mut infer_cache, table_expr.clone())
            else {
                continue;
            };

            let Some(rule) = resolve_expected_container_rule(db, &table_should_be) else {
                continue;
            };

            let actual_len = count_table_len(&rule.kind, &table_expr);
            if !rule.spec.contains_len(actual_len) {
                context.add_diagnostic(
                    DiagnosticCode::InvalidSizeValue,
                    table_expr.get_range(),
                    t!(
                        "Container size `%{size}` is out of range `%{range}`",
                        size = actual_len,
                        range = rule.spec.to_string()
                    )
                    .to_string(),
                    None,
                );
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
struct ContainerSizeRule {
    kind: ContainerKind,
    spec: SizeSpec,
}

fn resolve_expected_container_rule(_db: &DbIndex, ty: &LuaType) -> Option<ContainerSizeRule> {
    match ty {
        LuaType::Attributed(attributed) => {
            let spec = extract_size_spec_from_attributes(attributed.get_attributes().as_ref())?;
            resolve_container_kind(_db, attributed.get_base())
                .map(|kind| ContainerSizeRule { kind, spec })
        }
        LuaType::Union(union) => {
            let mut found: Option<ContainerSizeRule> = None;
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

fn extract_size_spec_from_attributes(
    attribute_uses: &[crate::LuaAttributeUse],
) -> Option<SizeSpec> {
    let attr = VSizeAttribute::find_in_uses(attribute_uses)?;
    attr.parse().ok()
}

fn resolve_container_kind(_db: &DbIndex, ty: &LuaType) -> Option<ContainerKind> {
    let ty = ty.strip_attributed();
    match ty {
        LuaType::Generic(generic) => match generic.get_base_type_id_ref().get_name() {
            "array" | "list" | "set" => Some(ContainerKind::ArrayLike),
            "map" => Some(ContainerKind::Map),
            _ => None,
        },
        LuaType::Array(_) => Some(ContainerKind::ArrayLike),
        LuaType::TableGeneric(params) => {
            if params.len() == 2 {
                Some(ContainerKind::Map)
            } else {
                None
            }
        }
        LuaType::Union(union) => {
            let mut found = None;
            for inner in union.into_vec().iter() {
                let Some(kind) = resolve_container_kind(_db, inner) else {
                    continue;
                };
                if found.is_some() {
                    return None;
                }
                found = Some(kind);
            }
            found
        }
        LuaType::MultiLineUnion(multi) => {
            let union = multi.to_union();
            resolve_container_kind(_db, &union)
        }
        _ => None,
    }
}

fn count_table_len(kind: &ContainerKind, table: &LuaTableExpr) -> usize {
    match kind {
        ContainerKind::ArrayLike => table
            .get_fields()
            .filter(|field| field.is_value_field())
            .count(),
        ContainerKind::Map => table.get_fields().count(),
    }
}
