use std::collections::HashSet;

use crate::{
    DiagnosticCode, LuaMemberKey, LuaType, LuaTypeDeclId, SemanticModel,
    attributes::TIndexAttribute,
    db_index::{DbIndex, LuaMemberOwner},
    diagnostic::checker::{Checker, DiagnosticContext},
    find_index_operations, is_sub_type_of,
};
use emmylua_parser::{
    LuaAstNode, LuaDocAttributeUse, LuaDocTagAttributeUse, LuaDocTagClass, LuaDocType,
    LuaLiteralToken,
};

pub struct InvalidIndexFieldChecker;

impl Checker for InvalidIndexFieldChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::InvalidIndexField];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let root = semantic_model.get_root().clone();
        let db = semantic_model.get_db();
        let file_id = semantic_model.get_file_id();

        // 获取当前文件中的所有类型声明
        let Some(file_types) = db.get_type_index().get_file_types(&file_id) else {
            return;
        };

        let config_table_type_id = LuaTypeDeclId::new(crate::CONFIG_TABLE_TYPE_NAME);

        // 收集所有继承自 ConfigTable 的类型 ID
        let mut config_table_ids: HashSet<LuaTypeDeclId> = HashSet::new();
        for type_decl_id in file_types.iter() {
            if is_sub_type_of(db, type_decl_id, &config_table_type_id) {
                config_table_ids.insert(type_decl_id.clone());
            }
        }

        if config_table_ids.is_empty() {
            return;
        }

        // 遍历所有属性使用
        for tag_use in root.descendants::<LuaDocTagAttributeUse>() {
            for attribute_use in tag_use.get_attribute_uses() {
                check_attribute_use(
                    context,
                    semantic_model,
                    &attribute_use,
                    &tag_use,
                    &config_table_ids,
                );
            }
        }
    }
}

fn check_attribute_use(
    context: &mut DiagnosticContext,
    semantic_model: &SemanticModel,
    attribute_use: &LuaDocAttributeUse,
    tag_use: &LuaDocTagAttributeUse,
    config_table_ids: &HashSet<LuaTypeDeclId>,
) -> Option<()> {
    // 检查是否为 t.index 属性
    let attr_type = attribute_use.get_type()?;
    let attr_name = attr_type.get_name_text()?;
    if attr_name != TIndexAttribute::NAME {
        return Some(());
    }

    // 找到此属性附加到的类型声明
    // @[t.index] 在 @class 之前，需要查找下一个兄弟节点 LuaDocTagClass
    let tag_class = find_next_class_sibling(tag_use)?;
    let class_name_token = tag_class.get_name_token()?;
    let class_name = class_name_token.get_name_text().to_string();
    let config_table_id = LuaTypeDeclId::new(&class_name);

    // 检查此类型是否是 ConfigTable
    if !config_table_ids.contains(&config_table_id) {
        return Some(());
    }

    // 获取 Bean 类型的成员列表
    let db = semantic_model.get_db();
    let bean_members = get_bean_member_names(db, &config_table_id)?;

    // 获取参数列表
    let arg_list = attribute_use.get_arg_list()?;
    let args: Vec<LuaDocType> = arg_list.get_args().collect();

    // 检查第一个参数 (indexs)
    let first_arg = args.first()?;

    // 提取字段名并检查
    let index_names = collect_index_names_from_doc_type(first_arg);
    for (name, range) in index_names {
        if !bean_members.contains(&name) {
            context.add_diagnostic(
                DiagnosticCode::InvalidIndexField,
                range,
                t!("Invalid index field `%{name}`", name = name).to_string(),
                None,
            );
        }
    }

    Some(())
}

/// 查找下一个兄弟节点中的 LuaDocTagClass
fn find_next_class_sibling(tag_use: &LuaDocTagAttributeUse) -> Option<LuaDocTagClass> {
    let mut next = tag_use.syntax().next_sibling();
    while let Some(sibling) = next {
        if let Some(class_tag) = LuaDocTagClass::cast(sibling.clone()) {
            return Some(class_tag);
        }
        next = sibling.next_sibling();
    }
    None
}

/// 获取 ConfigTable 的值类型（Bean）的所有字段名
fn get_bean_member_names(db: &DbIndex, config_table_id: &LuaTypeDeclId) -> Option<HashSet<String>> {
    // 获取 ConfigTable 的 [int] 成员 (Bean 类型)
    let config_table_type = LuaType::Ref(config_table_id.clone());
    let members = find_index_operations(db, &config_table_type)?;
    let int_member = members
        .iter()
        .find(|m| matches!(m.key, LuaMemberKey::ExprType(LuaType::Integer)))?;

    // 确定成员类型为 Bean
    let LuaType::Ref(bean_id) = &int_member.typ else {
        return None;
    };

    // 检查是否是 Bean 的子类型
    let bean_type_id = LuaTypeDeclId::new(crate::BEAN_TYPE_NAME);
    if !is_sub_type_of(db, bean_id, &bean_type_id) {
        return None;
    }

    // 获取 Bean 的成员列表
    let bean_members_refs = db
        .get_member_index()
        .get_members(&LuaMemberOwner::Type(bean_id.clone()))?;

    let mut names = HashSet::new();
    for member in bean_members_refs {
        if let LuaMemberKey::Name(name) = member.get_key() {
            names.insert(name.to_string());
        }
    }

    Some(names)
}

/// 从 LuaDocType 中提取索引字段名和其位置
fn collect_index_names_from_doc_type(doc_type: &LuaDocType) -> Vec<(String, rowan::TextRange)> {
    let mut results = Vec::new();
    collect_names_recursive(doc_type, &mut results);
    results
}

fn collect_names_recursive(doc_type: &LuaDocType, results: &mut Vec<(String, rowan::TextRange)>) {
    match doc_type {
        LuaDocType::Literal(literal) => {
            // 尝试从字面量获取字符串值
            if let Some(LuaLiteralToken::String(string_token)) = literal.get_literal() {
                let value = string_token.get_value();
                results.push((value, literal.get_range()));
            }
        }
        LuaDocType::Tuple(tuple) => {
            for item in tuple.get_types() {
                collect_names_recursive(&item, results);
            }
        }
        LuaDocType::Binary(binary) => {
            // Binary 可能是 Union (|) 操作
            if let Some((left, right)) = binary.get_types() {
                collect_names_recursive(&left, results);
                collect_names_recursive(&right, results);
            }
        }
        LuaDocType::Array(array) => {
            if let Some(inner) = array.get_type() {
                collect_names_recursive(&inner, results);
            }
        }
        _ => {}
    }
}
