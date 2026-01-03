use std::collections::HashSet;

use crate::{
    DiagnosticCode, DocTypeInferContext, LuaType, SemanticModel, TypeCheckFailReason,
    TypeCheckResult, attributes::parse_set_spec_type, diagnostic::checker::humanize_lint_type,
    infer_doc_type,
};
use emmylua_parser::{LuaAstNode, LuaDocAttributeUse, LuaDocTagAttributeUse, LuaDocType};
use rowan::TextRange;

use super::{Checker, DiagnosticContext};

pub struct AttributeCheckChecker;

impl Checker for AttributeCheckChecker {
    const CODES: &[DiagnosticCode] = &[
        DiagnosticCode::AttributeParamTypeMismatch,
        DiagnosticCode::AttributeMissingParameter,
        DiagnosticCode::AttributeRedundantParameter,
    ];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let root = semantic_model.get_root().clone();
        for tag_use in root.descendants::<LuaDocTagAttributeUse>() {
            for attribute_use in tag_use.get_attribute_uses() {
                check_attribute_use(context, semantic_model, &attribute_use);
            }
        }
    }
}

fn check_attribute_use(
    context: &mut DiagnosticContext,
    semantic_model: &SemanticModel,
    attribute_use: &LuaDocAttributeUse,
) -> Option<()> {
    let doc_ctx = DocTypeInferContext::new(semantic_model.get_db(), semantic_model.get_file_id());
    let attribute_type = infer_doc_type(doc_ctx, &LuaDocType::Name(attribute_use.get_type()?));
    let LuaType::Ref(type_id) = attribute_type else {
        return None;
    };
    let type_decl = semantic_model
        .get_db()
        .get_type_index()
        .get_type_decl(&type_id)?;
    if !type_decl.is_attribute() {
        return None;
    }
    let LuaType::DocAttribute(attr_def) = type_decl.get_attribute_type()? else {
        return None;
    };

    let def_params = attr_def.get_params();
    let arg_list = attribute_use.get_arg_list();
    let args = match &arg_list {
        Some(arg_list) => arg_list.get_args().collect::<Vec<_>>(),
        None => vec![],
    };
    check_param_count(context, &def_params, &attribute_use, &args);

    if type_id.get_name() == "v.set" {
        check_vset_values_param(context, semantic_model, attribute_use, &args);
        return Some(());
    }
    check_param(context, semantic_model, &def_params, args);

    Some(())
}

fn check_vset_values_param(
    context: &mut DiagnosticContext,
    semantic_model: &SemanticModel,
    attribute_use: &LuaDocAttributeUse,
    args: &[LuaDocType],
) -> Option<()> {
    let Some(first_arg) = args.first() else {
        let Some(arg_list) = attribute_use.get_arg_list() else {
            return Some(());
        };

        let raw = arg_list.syntax().text().to_string();
        let raw = raw.trim();
        let inner = raw
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(raw)
            .trim();

        if inner.is_empty() {
            return Some(());
        }

        let normalized: String = inner.chars().filter(|c| !c.is_whitespace()).collect();
        let reason = if normalized == "[]" {
            "values must not be empty"
        } else {
            "values parameter must be a tuple literal like [1, 2]"
        };

        context.add_diagnostic(
            DiagnosticCode::AttributeParamTypeMismatch,
            arg_list.get_range(),
            t!("Invalid v.set values: %{reason}", reason = reason).to_string(),
            None,
        );

        return Some(());
    };
    let doc_ctx = DocTypeInferContext::new(semantic_model.get_db(), semantic_model.get_file_id());
    let arg_type = infer_doc_type(doc_ctx, first_arg);
    if let Err(err) = parse_set_spec_type(&arg_type) {
        context.add_diagnostic(
            DiagnosticCode::AttributeParamTypeMismatch,
            first_arg.get_range(),
            t!("Invalid v.set values: %{reason}", reason = err.to_string()).to_string(),
            None,
        );
    }

    Some(())
}

/// 检查参数数量是否匹配
fn check_param_count(
    context: &mut DiagnosticContext,
    def_params: &[(String, Option<LuaType>)],
    attribute_use: &LuaDocAttributeUse,
    args: &[LuaDocType],
) -> Option<()> {
    let call_args_count = args.len();
    // 调用参数少于定义参数, 需要考虑可空参数
    if call_args_count < def_params.len() {
        for def_param in def_params[call_args_count..].iter() {
            if def_param.0 == "..." {
                break;
            }
            if def_param.1.as_ref().is_some_and(is_nullable) {
                continue;
            }
            context.add_diagnostic(
                DiagnosticCode::AttributeMissingParameter,
                match args.last() {
                    Some(arg) => arg.get_range(),
                    None => attribute_use.get_range(),
                },
                t!(
                    "expected %{num} parameters but found %{found_num}",
                    num = def_params.len(),
                    found_num = call_args_count
                )
                .to_string(),
                None,
            );
        }
    }
    // 调用参数多于定义参数, 需要考虑可变参数
    else if call_args_count > def_params.len() {
        // 参数定义中最后一个参数是 `...`
        if def_params.last().is_some_and(|(name, typ)| {
            name == "..." || typ.as_ref().is_some_and(|typ| typ.is_variadic())
        }) {
            return Some(());
        }
        for arg in args[def_params.len()..].iter() {
            context.add_diagnostic(
                DiagnosticCode::AttributeRedundantParameter,
                arg.get_range(),
                t!(
                    "expected %{num} parameters but found %{found_num}",
                    num = def_params.len(),
                    found_num = call_args_count
                )
                .to_string(),
                None,
            );
        }
    }

    Some(())
}

/// 检查参数是否匹配
fn check_param(
    context: &mut DiagnosticContext,
    semantic_model: &SemanticModel,
    def_params: &[(String, Option<LuaType>)],
    args: Vec<LuaDocType>,
) -> Option<()> {
    let doc_ctx = DocTypeInferContext::new(semantic_model.get_db(), semantic_model.get_file_id());
    let mut call_arg_types = Vec::new();
    for arg in &args {
        let arg_type = infer_doc_type(doc_ctx, arg);
        call_arg_types.push(arg_type);
    }

    for (idx, param) in def_params.iter().enumerate() {
        if param.0 == "..." {
            if call_arg_types.len() < idx {
                break;
            }
            if let Some(variadic_type) = param.1.clone() {
                for arg_type in call_arg_types[idx..].iter() {
                    let result = semantic_model.type_check_detail(&variadic_type, arg_type);
                    if result.is_err() {
                        add_type_check_diagnostic(
                            context,
                            semantic_model,
                            args.get(idx)?.get_range(),
                            &variadic_type,
                            arg_type,
                            result,
                        );
                    }
                }
            }
            break;
        }
        if let Some(param_type) = param.1.clone() {
            let arg_type = call_arg_types.get(idx).unwrap_or(&LuaType::Any);
            let result = semantic_model.type_check_detail(&param_type, arg_type);
            if result.is_err() {
                add_type_check_diagnostic(
                    context,
                    semantic_model,
                    args.get(idx)?.get_range(),
                    &param_type,
                    arg_type,
                    result,
                );
            }
        }
    }
    Some(())
}

fn add_type_check_diagnostic(
    context: &mut DiagnosticContext,
    semantic_model: &SemanticModel,
    range: TextRange,
    param_type: &LuaType,
    expr_type: &LuaType,
    result: TypeCheckResult,
) {
    let db = semantic_model.get_db();
    match result {
        Ok(_) => (),
        Err(reason) => {
            let reason_message = match reason {
                TypeCheckFailReason::TypeNotMatchWithReason(reason) => reason,
                TypeCheckFailReason::TypeNotMatch | TypeCheckFailReason::DonotCheck => {
                    "".to_string()
                }
                TypeCheckFailReason::TypeRecursion => "type recursion".to_string(),
            };
            context.add_diagnostic(
                DiagnosticCode::AttributeParamTypeMismatch,
                range,
                t!(
                    "expected `%{source}` but found `%{found}`. %{reason}",
                    source = humanize_lint_type(db, param_type),
                    found = humanize_lint_type(db, expr_type),
                    reason = reason_message
                )
                .to_string(),
                None,
            );
        }
    }
}

fn is_nullable(typ: &LuaType) -> bool {
    let mut stack: Vec<LuaType> = Vec::new();
    stack.push(typ.clone());
    let mut visited = HashSet::new();
    while let Some(typ) = stack.pop() {
        if visited.contains(&typ) {
            continue;
        }
        visited.insert(typ.clone());
        match typ {
            LuaType::Any | LuaType::Unknown | LuaType::Nil => return true,
            LuaType::Union(u) => {
                for t in u.into_vec() {
                    stack.push(t);
                }
            }
            _ => {}
        }
    }
    false
}
