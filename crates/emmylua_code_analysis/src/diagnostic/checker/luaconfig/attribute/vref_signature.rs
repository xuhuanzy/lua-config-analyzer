use emmylua_parser::{
    LuaAstNode, LuaDocAttributeUse, LuaDocTagAttributeUse, LuaDocType, LuaLiteralToken,
};

use crate::{
    DiagnosticCode, LuaMemberKey, LuaTypeDeclId, SemanticModel,
    attributes::ConfigTableMode,
    diagnostic::checker::{Checker, DiagnosticContext},
    semantic::shared::luaconfig::CONFIG_TABLE,
};

pub struct VRefSignatureChecker;

impl Checker for VRefSignatureChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::InvalidRefSignature];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let file_id = semantic_model.get_file_id();
        let db = semantic_model.get_db();
        let root = semantic_model.get_root().clone();

        for tag_use in root.descendants::<LuaDocTagAttributeUse>() {
            for attribute_use in tag_use.get_attribute_uses() {
                if !is_vref_attribute_use(&attribute_use) {
                    continue;
                }

                let Some((table_name, field_name)) = extract_vref_signature_args(&attribute_use)
                else {
                    continue;
                };

                match parse_vref_signature(db, file_id, &table_name, field_name.as_deref()) {
                    Ok(_) | Err(VRefSignatureError::UnsupportedSingleton) => {}
                    Err(err) => {
                        context.add_diagnostic(
                            DiagnosticCode::InvalidRefSignature,
                            attribute_use.get_range(),
                            err.to_message(),
                            None,
                        );
                    }
                }
            }
        }
    }
}

fn is_vref_attribute_use(attribute_use: &LuaDocAttributeUse) -> bool {
    attribute_use
        .get_type()
        .and_then(|ty| ty.get_name_token())
        .is_some_and(|token| token.get_name_text() == "v.ref")
}

fn extract_vref_signature_args(
    attribute_use: &LuaDocAttributeUse,
) -> Option<(String, Option<String>)> {
    let args = attribute_use.get_arg_list()?.get_args().collect::<Vec<_>>();
    let table_name = doc_type_string_literal(args.first()?)?;

    let field_name = match args.get(1) {
        None => None,
        Some(field) => Some(doc_type_string_literal(field)?),
    };

    Some((table_name, field_name))
}

fn doc_type_string_literal(ty: &LuaDocType) -> Option<String> {
    let LuaDocType::Literal(literal) = ty else {
        return None;
    };

    match literal.get_literal()? {
        LuaLiteralToken::String(token) => Some(token.get_value()),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub(in crate::diagnostic::checker::luaconfig) enum VRefSignatureError {
    UnknownConfigTable { table: String },
    NotConfigTable { table: String },
    NoPrimaryKeys { table: LuaTypeDeclId },
    MapMustHaveExactlyOnePrimaryKey { table: LuaTypeDeclId },
    MapNonNamePrimaryKey { table: LuaTypeDeclId },
    MapPrimaryKeyMismatch { table: LuaTypeDeclId, pk: String },
    ListRequiresField { table: LuaTypeDeclId },
    FieldNotPrimaryKey { table: LuaTypeDeclId, field: String },
    UnsupportedSingleton,
}

impl VRefSignatureError {
    fn to_message(&self) -> String {
        match self {
            VRefSignatureError::UnknownConfigTable { table } => t!(
                "Invalid v.ref: unknown config table `%{table}`",
                table = table
            )
            .to_string(),
            VRefSignatureError::NotConfigTable { table } => t!(
                "Invalid v.ref: `%{table}` is not a `ConfigTable`",
                table = table
            )
            .to_string(),
            VRefSignatureError::NoPrimaryKeys { table } => t!(
                "Invalid v.ref: `%{table}` has no primary keys",
                table = table.get_name()
            )
            .to_string(),
            VRefSignatureError::MapMustHaveExactlyOnePrimaryKey { table } => t!(
                "Invalid v.ref: map table `%{table}` must have exactly one primary key",
                table = table.get_name()
            )
            .to_string(),
            VRefSignatureError::MapNonNamePrimaryKey { table } => t!(
                "Invalid v.ref: map table `%{table}` has non-name primary key",
                table = table.get_name()
            )
            .to_string(),
            VRefSignatureError::MapPrimaryKeyMismatch { table, pk } => t!(
                "Invalid v.ref: map table `%{table}` primary key is `%{pk}`",
                table = table.get_name(),
                pk = pk
            )
            .to_string(),
            VRefSignatureError::ListRequiresField { table } => t!(
                "Invalid v.ref: list table `%{table}` requires explicit `field`",
                table = table.get_name()
            )
            .to_string(),
            VRefSignatureError::FieldNotPrimaryKey { table, field } => t!(
                "Invalid v.ref: `%{field}` is not a primary key of `%{table}`",
                field = field,
                table = table.get_name()
            )
            .to_string(),
            VRefSignatureError::UnsupportedSingleton => String::new(),
        }
    }
}

pub(in crate::diagnostic::checker::luaconfig) fn parse_vref_signature(
    db: &crate::DbIndex,
    file_id: crate::FileId,
    target_table_name: &str,
    target_field_name: Option<&str>,
) -> Result<(LuaTypeDeclId, LuaMemberKey), VRefSignatureError> {
    let Some(target_decl) = db
        .get_type_index()
        .find_type_decl(file_id, target_table_name)
    else {
        return Err(VRefSignatureError::UnknownConfigTable {
            table: target_table_name.to_string(),
        });
    };

    let target_table_id = target_decl.get_id();
    if !CONFIG_TABLE.is_config_table(db, &target_table_id) {
        return Err(VRefSignatureError::NotConfigTable {
            table: target_table_name.to_string(),
        });
    }

    let mode = db
        .get_config_index()
        .get_config_table_mode(&target_table_id);
    if mode == ConfigTableMode::Singleton {
        return Err(VRefSignatureError::UnsupportedSingleton);
    }

    let Some(index_keys) = db
        .get_config_index()
        .get_config_table_keys(&target_table_id)
    else {
        return Err(VRefSignatureError::NoPrimaryKeys {
            table: target_table_id,
        });
    };

    let keys = index_keys.keys();
    match mode {
        ConfigTableMode::Map => {
            if keys.len() != 1 {
                return Err(VRefSignatureError::MapMustHaveExactlyOnePrimaryKey {
                    table: target_table_id,
                });
            }

            let pk = keys[0].clone();
            if let Some(field_name) = target_field_name {
                let Some(pk_name) = pk.get_name() else {
                    return Err(VRefSignatureError::MapNonNamePrimaryKey {
                        table: target_table_id,
                    });
                };
                if pk_name != field_name {
                    return Err(VRefSignatureError::MapPrimaryKeyMismatch {
                        table: target_table_id,
                        pk: pk_name.to_string(),
                    });
                }
            }

            Ok((target_table_id, pk))
        }
        ConfigTableMode::List => {
            let Some(field_name) = target_field_name else {
                return Err(VRefSignatureError::ListRequiresField {
                    table: target_table_id,
                });
            };

            let field_key = LuaMemberKey::Name(field_name.to_string().into());
            if !keys.iter().any(|k| k == &field_key) {
                return Err(VRefSignatureError::FieldNotPrimaryKey {
                    table: target_table_id,
                    field: field_name.to_string(),
                });
            }

            Ok((target_table_id, field_key))
        }
        ConfigTableMode::Singleton => Err(VRefSignatureError::UnsupportedSingleton),
    }
}
