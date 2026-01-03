use emmylua_parser::{
    LuaAstNode, LuaDocAttributeUse, LuaDocTagAttributeUse, LuaDocType, LuaLiteralToken,
};

use crate::{
    DiagnosticCode, SemanticModel,
    attributes::{RangeSpec, parse_range_spec},
    diagnostic::checker::{Checker, DiagnosticContext},
};

pub struct VRangeSignatureChecker;

impl Checker for VRangeSignatureChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::InvalidRangeSignature];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let root = semantic_model.get_root().clone();

        for tag_use in root.descendants::<LuaDocTagAttributeUse>() {
            for attribute_use in tag_use.get_attribute_uses() {
                if !is_vrange_attribute_use(&attribute_use) {
                    continue;
                }

                let Some(spec) = parse_vrange_signature(&attribute_use) else {
                    continue;
                };

                if let Err(err) = validate_vrange_spec(&spec) {
                    context.add_diagnostic(
                        DiagnosticCode::InvalidRangeSignature,
                        attribute_use.get_range(),
                        t!(
                            "Invalid v.range signature: %{reason}",
                            reason = err.to_string()
                        )
                        .to_string(),
                        None,
                    );
                }
            }
        }
    }
}

fn is_vrange_attribute_use(attribute_use: &LuaDocAttributeUse) -> bool {
    attribute_use
        .get_type()
        .and_then(|ty| ty.get_name_token())
        .is_some_and(|token| token.get_name_text() == "v.range")
}

fn parse_vrange_signature(attribute_use: &LuaDocAttributeUse) -> Option<Result<RangeSpec, String>> {
    let args = attribute_use
        .get_arg_list()
        .map(|l| l.get_args().collect::<Vec<_>>())
        .unwrap_or_default();

    match args.as_slice() {
        [a] => {
            if let Some(s) = doc_type_string_literal(a) {
                return Some(parse_range_spec(&s).map_err(|e| e.to_string()));
            }
            if let Some(n) = doc_type_number_literal(a) {
                return Some(Ok(RangeSpec::exact(n)));
            }
            None
        }
        _ => None,
    }
}

fn validate_vrange_spec(spec: &Result<RangeSpec, String>) -> Result<(), String> {
    match spec {
        Ok(_) => Ok(()),
        Err(err) => Err(err.clone()),
    }
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

fn doc_type_number_literal(ty: &LuaDocType) -> Option<f64> {
    let LuaDocType::Literal(literal) = ty else {
        return None;
    };

    match literal.get_literal()? {
        LuaLiteralToken::Number(token) => match token.get_number_value() {
            emmylua_parser::NumberResult::Int(i) => Some(i as f64),
            emmylua_parser::NumberResult::Uint(u) => Some(u as f64),
            emmylua_parser::NumberResult::Float(f) => Some(f),
        },
        _ => None,
    }
}
