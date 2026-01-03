use emmylua_parser::{
    LuaAstNode, LuaDocAttributeUse, LuaDocTagAttributeUse, LuaDocType, LuaLiteralToken,
    NumberResult,
};
use rowan::NodeOrToken;

use crate::{
    DiagnosticCode, SemanticModel,
    attributes::parse_range_spec,
    diagnostic::checker::{Checker, DiagnosticContext},
};

pub struct VSizeSignatureChecker;

impl Checker for VSizeSignatureChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::InvalidSizeSignature];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let root = semantic_model.get_root().clone();

        for tag_use in root.descendants::<LuaDocTagAttributeUse>() {
            let target_type = resolve_target_doc_type(&tag_use);
            for attribute_use in tag_use.get_attribute_uses() {
                if !is_vsize_attribute_use(&attribute_use) {
                    continue;
                }

                if let Err(reason) = validate_vsize_signature(&attribute_use, target_type.as_ref())
                {
                    context.add_diagnostic(
                        DiagnosticCode::InvalidSizeSignature,
                        attribute_use.get_range(),
                        t!("Invalid v.size: %{reason}", reason = reason).to_string(),
                        None,
                    );
                }
            }
        }
    }
}

fn is_vsize_attribute_use(attribute_use: &LuaDocAttributeUse) -> bool {
    attribute_use
        .get_type()
        .and_then(|ty| ty.get_name_token())
        .is_some_and(|token| token.get_name_text() == "v.size")
}

fn validate_vsize_signature(
    attribute_use: &LuaDocAttributeUse,
    target_type: Option<&LuaDocType>,
) -> Result<(), String> {
    if target_type.is_none() {
        return Err(
            "v.size must be used as a type attribute (e.g. ([v.size(1)] array<integer>))"
                .to_string(),
        );
    }

    let target_type = target_type.unwrap();
    if !is_container_doc_type(target_type) {
        return Err("v.size can only be applied to container types".to_string());
    }

    let args = attribute_use
        .get_arg_list()
        .map(|l| l.get_args().collect::<Vec<_>>())
        .unwrap_or_default();

    if args.len() != 1 {
        return Err("v.size expects exactly one parameter".to_string());
    }

    parse_size_spec_doc_type(&args[0]).map(|_| ())
}

fn parse_size_spec_doc_type(arg: &LuaDocType) -> Result<(), String> {
    if let Some(s) = doc_type_string_literal(arg) {
        let spec = parse_range_spec(&s).map_err(|e| e.to_string())?;
        validate_size_range(&spec)?;
        return Ok(());
    }

    if let Some(n) = doc_type_number_literal(arg) {
        if n.fract() != 0.0 {
            return Err("size must be an integer".to_string());
        }
        if n < 0.0 {
            return Err("size must be >= 0".to_string());
        }
        return Ok(());
    }

    Err("size parameter must be a number or a string range".to_string())
}

fn validate_size_range(spec: &crate::attributes::RangeSpec) -> Result<(), String> {
    if let Some(min) = spec.min {
        if !min.is_finite() || min.fract() != 0.0 {
            return Err("size min must be an integer".to_string());
        }
        if min < 0.0 {
            return Err("size min must be >= 0".to_string());
        }
    }

    if let Some(max) = spec.max {
        if !max.is_finite() || max.fract() != 0.0 {
            return Err("size max must be an integer".to_string());
        }
        if max < 0.0 {
            return Err("size max must be >= 0".to_string());
        }
    }

    Ok(())
}

fn is_container_doc_type(ty: &LuaDocType) -> bool {
    match ty {
        LuaDocType::Array(_) => true,
        LuaDocType::Generic(generic) => generic
            .get_name_type()
            .and_then(|name_type| name_type.get_name_text())
            .is_some_and(|name| {
                matches!(name.as_str(), "array" | "list" | "set" | "map" | "table")
            }),
        _ => false,
    }
}

fn resolve_target_doc_type(tag_use: &LuaDocTagAttributeUse) -> Option<LuaDocType> {
    let mut cursor = tag_use.syntax().clone().next_sibling_or_token();
    while let Some(element) = cursor {
        match element {
            NodeOrToken::Token(token) => {
                if token.text().trim().is_empty() {
                    cursor = token.next_sibling_or_token();
                    continue;
                }
                return None;
            }
            NodeOrToken::Node(node) => {
                if LuaDocType::can_cast(node.kind().into()) {
                    return LuaDocType::cast(node);
                }
                return None;
            }
        }
    }
    None
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
            NumberResult::Int(i) => Some(i as f64),
            NumberResult::Uint(u) => Some(u as f64),
            NumberResult::Float(f) => Some(f),
        },
        _ => None,
    }
}
