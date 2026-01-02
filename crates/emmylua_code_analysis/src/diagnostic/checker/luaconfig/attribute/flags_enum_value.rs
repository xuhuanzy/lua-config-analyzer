use crate::{
    DbIndex, DiagnosticCode, LuaMemberKey, LuaType,
    attributes::is_flags_attribute,
    db_index::{LuaMember, LuaMemberOwner, LuaSemanticDeclId},
    diagnostic::checker::{Checker, DiagnosticContext},
    semantic::SemanticModel,
};

pub struct FlagsEnumValueChecker;

impl Checker for FlagsEnumValueChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::InvalidFlagsEnumValue];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let db = semantic_model.get_db();
        let file_id = semantic_model.get_file_id();

        let Some(file_types) = db.get_type_index().get_file_types(&file_id) else {
            return;
        };

        for type_decl_id in file_types {
            let Some(type_decl) = db.get_type_index().get_type_decl(type_decl_id) else {
                continue;
            };

            if !type_decl.is_enum() || type_decl.is_enum_key() {
                continue;
            }

            let owner_id = LuaSemanticDeclId::TypeDecl(type_decl_id.clone());
            let Some(property) = db.get_property_index().get_property(&owner_id) else {
                continue;
            };

            if !is_flags_attribute(property) {
                continue;
            }

            let Some(enum_members) = db
                .get_member_index()
                .get_members(&LuaMemberOwner::Type(type_decl_id.clone()))
            else {
                continue;
            };

            for member in enum_members {
                let Some(value) = get_member_integer_value(db, member) else {
                    continue;
                };

                if value == 0 {
                    continue;
                }

                if !is_power_of_two(value) {
                    context.add_diagnostic(
                        DiagnosticCode::InvalidFlagsEnumValue,
                        member.get_range(),
                        t!(
                            "Flags enum `%{enum_name}` field `%{name}` value `%{value}` must be a power of two",
                            enum_name = type_decl_id.get_name(),
                            name = member_key_to_string(member.get_key()),
                            value = value
                        )
                        .to_string(),
                        None,
                    );
                }
            }
        }
    }
}

fn get_member_integer_value(db: &DbIndex, member: &LuaMember) -> Option<i64> {
    let type_cache = db
        .get_type_index()
        .get_type_cache(&member.get_id().into())?;
    match type_cache.as_type() {
        LuaType::IntegerConst(i) | LuaType::DocIntegerConst(i) => Some(*i),
        _ => None,
    }
}

fn member_key_to_string(key: &LuaMemberKey) -> String {
    match key {
        LuaMemberKey::Name(name) => name.to_string(),
        LuaMemberKey::Integer(i) => i.to_string(),
        LuaMemberKey::ExprType(_) | LuaMemberKey::None => key.to_path(),
    }
}

fn is_power_of_two(value: i64) -> bool {
    value > 0 && (value & (value - 1)) == 0
}
