use emmylua_parser::{LuaAstNode, LuaDocAttributeUse, LuaDocTagAttributeUse, LuaDocType};
use rowan::NodeOrToken;

use crate::{
    DiagnosticCode, SemanticModel,
    diagnostic::checker::{Checker, DiagnosticContext},
};

pub struct VSetSignatureChecker;

impl Checker for VSetSignatureChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::InvalidSetSignature];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let root = semantic_model.get_root().clone();

        for tag_use in root.descendants::<LuaDocTagAttributeUse>() {
            let target_type = resolve_target_doc_type(&tag_use);
            for attribute_use in tag_use.get_attribute_uses() {
                if !is_vset_attribute_use(&attribute_use) {
                    continue;
                }

                if target_type.is_none() {
                    context.add_diagnostic(
                        DiagnosticCode::InvalidSetSignature,
                        attribute_use.get_range(),
                        t!(
                            "Invalid v.set: v.set must be used as a type attribute (e.g. array<[v.set([1, 2])] integer>)"
                        )
                        .to_string(),
                        None,
                    );
                }
            }
        }
    }
}

fn is_vset_attribute_use(attribute_use: &LuaDocAttributeUse) -> bool {
    attribute_use
        .get_type()
        .and_then(|ty| ty.get_name_token())
        .is_some_and(|token| token.get_name_text() == "v.set")
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
