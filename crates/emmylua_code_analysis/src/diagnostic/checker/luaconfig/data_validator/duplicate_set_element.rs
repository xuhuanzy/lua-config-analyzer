use std::collections::HashSet;

use emmylua_parser::{LuaAstNode, LuaTableExpr};

use crate::{
    DbIndex, DiagnosticCode, LuaType, RenderLevel, SemanticModel,
    diagnostic::checker::{Checker, DiagnosticContext},
    humanize_type, infer_expr, infer_table_should_be,
};

pub struct DuplicateSetElementChecker;

impl Checker for DuplicateSetElementChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::DuplicateSetElement];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let db = semantic_model.get_db();
        let root = semantic_model.get_root().clone();

        let mut infer_cache = semantic_model.get_cache().borrow_mut();

        for table_expr in root.descendants::<LuaTableExpr>() {
            let Ok(table_should_be) =
                infer_table_should_be(db, &mut infer_cache, table_expr.clone())
            else {
                continue;
            };

            if !is_set_type(db, &table_should_be) {
                continue;
            }

            // set 是数组语义，仅检查 value_field 的重复。
            let mut seen: HashSet<LuaType> = HashSet::new();
            let mut reported: HashSet<LuaType> = HashSet::new();

            for field in table_expr.get_fields() {
                if !field.is_value_field() {
                    continue;
                }

                let Some(value_expr) = field.get_value_expr() else {
                    continue;
                };

                let Ok(value_typ) = infer_expr(db, &mut infer_cache, value_expr.clone()) else {
                    continue;
                };

                if !is_checkable_literal_key(&value_typ) {
                    continue;
                }

                if !seen.insert(value_typ.clone()) {
                    // 对每个重复值只提示一次（但定位到重复项处），避免大量噪音。
                    if !reported.insert(value_typ.clone()) {
                        continue;
                    }

                    let value = humanize_type(db, &value_typ, RenderLevel::Simple);
                    context.add_diagnostic(
                        DiagnosticCode::DuplicateSetElement,
                        field.get_range(),
                        t!("Duplicate set element value `%{value}`", value = value).to_string(),
                        None,
                    );
                }
            }
        }
    }
}

fn is_set_type(db: &DbIndex, ty: &LuaType) -> bool {
    let ty = ty.strip_attributed();
    match ty {
        LuaType::Generic(generic) => generic.get_base_type_id_ref().get_name() == "set",
        LuaType::Union(union) => {
            let mut found = false;
            for inner in union.into_vec().iter() {
                if is_set_type(db, inner) {
                    if found {
                        return false;
                    }
                    found = true;
                }
            }
            found
        }
        LuaType::MultiLineUnion(multi) => {
            let union = multi.to_union();
            is_set_type(db, &union)
        }
        LuaType::Ref(id) => db
            .get_type_index()
            .get_type_decl(id)
            .is_some_and(|decl| decl.get_full_name() == "set"),
        LuaType::Def(id) => db
            .get_type_index()
            .get_type_decl(id)
            .is_some_and(|decl| decl.get_full_name() == "set"),
        _ => false,
    }
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
