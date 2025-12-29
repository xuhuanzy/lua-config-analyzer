use std::collections::HashMap;
use std::ops::Deref;

use emmylua_parser::{LuaAst, LuaAstNode, LuaTableExpr};
use rowan::TextRange;

use crate::{
    DiagnosticCode, LuaMemberKey, LuaMemberOwner, LuaSemanticDeclId, LuaType, LuaTypeDeclId,
    RenderLevel, SemanticModel,
    diagnostic::checker::{Checker, DiagnosticContext},
    find_index_operations, humanize_type,
    semantic::attributes::{ConfigTableIndexMode, TIndexAttribute},
};

/* 检查主键是否重复 */

pub struct DuplicatePrimaryKeyChecker;

impl Checker for DuplicatePrimaryKeyChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::DuplicatePrimaryKey];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let root = semantic_model.get_root().clone();
        // 缓存已处理的配置表范围，嵌套在这些范围内的表将被跳过
        let mut checked_ranges: Vec<TextRange> = Vec::new();
        for table in root.descendants::<LuaTableExpr>() {
            check_duplicate_primary_key(context, semantic_model, table, &mut checked_ranges);
        }
    }
}

// ConfigTableIndexMode 已移至 semantic::attributes 模块

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigTableIndexKeys {
    Solo(Vec<LuaMemberKey>),
    Union(Vec<LuaMemberKey>),
}

impl ConfigTableIndexKeys {
    fn new(keys: Vec<LuaMemberKey>, mode: ConfigTableIndexMode) -> Option<Self> {
        if keys.is_empty() {
            return None;
        }

        if keys.len() == 1 {
            return Some(Self::Solo(keys));
        }

        Some(match mode {
            ConfigTableIndexMode::Solo => Self::Solo(keys),
            ConfigTableIndexMode::Union => Self::Union(keys),
        })
    }

    fn keys(&self) -> &[LuaMemberKey] {
        match self {
            Self::Solo(keys) | Self::Union(keys) => keys,
        }
    }
}

/**
 * 获取配置表的主键
 */
pub fn get_config_table_keys(
    semantic_model: &SemanticModel,
    table: &LuaTableExpr,
) -> Option<ConfigTableIndexKeys> {
    let db = semantic_model.get_db();
    let table_type = semantic_model.infer_table_should_be(table.clone())?;
    let LuaType::Ref(config_table) = table_type else {
        return None;
    };

    if !semantic_model.is_sub_type_of(&config_table, &LuaTypeDeclId::new("ConfigTable")) {
        return None;
    }

    let members =
        find_index_operations(semantic_model.get_db(), &LuaType::Ref(config_table.clone()))?;
    let members = members
        .iter()
        .filter(|member| matches!(member.key, LuaMemberKey::ExprType(LuaType::Integer)))
        .collect::<Vec<_>>();
    let member = members.first()?;
    // 确定成员类型为 Bean
    if let LuaType::Ref(bean) = &member.typ {
        if !semantic_model.is_sub_type_of(bean, &LuaTypeDeclId::new("Bean")) {
            return None;
        }
        let mut members = semantic_model
            .get_db()
            .get_member_index()
            .get_members(&LuaMemberOwner::Type(bean.clone()))?
            .to_vec();
        let property = db
            .get_property_index()
            .get_property(&LuaSemanticDeclId::TypeDecl(config_table.clone()))?;

        let Some(index_attr) = TIndexAttribute::find_in(property) else {
            // 根据 member_id 的位置排序, 确保顺序稳定
            members.sort_by_key(|m| m.get_sort_key());
            let default_index = members.first()?.get_key().clone();
            return ConfigTableIndexKeys::new(vec![default_index], ConfigTableIndexMode::Union);
        };

        let (keys, mode) = resolve_config_table_index_from_attr(&index_attr, &members);
        let keys = if keys.is_empty() {
            // 根据 member_id 的位置排序, 确保顺序稳定
            members.sort_by_key(|m| m.get_sort_key());
            let default_index = members.first()?.get_key().clone();
            vec![default_index]
        } else {
            keys
        };

        return ConfigTableIndexKeys::new(keys, mode);
    }

    None
}

fn resolve_config_table_index_from_attr(
    index_attr: &TIndexAttribute,
    bean_members: &[&crate::LuaMember],
) -> (Vec<LuaMemberKey>, ConfigTableIndexMode) {
    let mut keys = index_attr
        .get_indexs()
        .map(collect_index_member_keys_from_type)
        .unwrap_or_default();

    // 过滤掉不存在的成员
    keys.retain(|key| bean_members.iter().any(|m| m.get_key() == key));

    // 去重
    let mut uniq = Vec::with_capacity(keys.len());
    for k in keys {
        if !uniq.contains(&k) {
            uniq.push(k);
        }
    }

    let mode = if uniq.len() > 1 {
        index_attr.get_mode()
    } else {
        ConfigTableIndexMode::Union
    };

    (uniq, mode)
}

fn collect_index_member_keys_from_type(ty: &LuaType) -> Vec<LuaMemberKey> {
    collect_index_member_names_from_type(ty)
        .into_iter()
        .map(LuaMemberKey::Name)
        .collect()
}

fn collect_index_member_names_from_type(ty: &LuaType) -> Vec<smol_str::SmolStr> {
    match ty {
        LuaType::DocStringConst(s) | LuaType::StringConst(s) => vec![s.deref().clone()],
        LuaType::Tuple(tuple) => tuple
            .get_types()
            .iter()
            .flat_map(collect_index_member_names_from_type)
            .collect(),
        LuaType::Union(union) => union
            .into_vec()
            .iter()
            .flat_map(collect_index_member_names_from_type)
            .collect(),
        _ => Vec::new(),
    }
}

fn check_duplicate_primary_key(
    context: &mut DiagnosticContext,
    semantic_model: &SemanticModel,
    table: LuaTableExpr,
    checked_ranges: &mut Vec<TextRange>,
) -> Option<()> {
    let table_range = table.get_range();

    // 检查当前表是否在已处理的配置表范围内
    if checked_ranges.iter().any(|r| r.contains_range(table_range)) {
        return None;
    }

    let index_keys = get_config_table_keys(semantic_model, &table)?;

    // 成功获取 index_keys, 将此表范围添加到缓存
    checked_ranges.push(table_range);

    let fields = table.get_fields().collect::<Vec<_>>();

    match index_keys {
        ConfigTableIndexKeys::Solo(keys) => {
            check_duplicate_primary_key_solo(context, semantic_model, &fields, &keys)?;
        }
        ConfigTableIndexKeys::Union(keys) => {
            check_duplicate_primary_key_union(context, semantic_model, &fields, &keys)?;
        }
    }

    Some(())
}

fn check_duplicate_primary_key_solo(
    context: &mut DiagnosticContext,
    semantic_model: &SemanticModel,
    fields: &[emmylua_parser::LuaTableField],
    keys: &[LuaMemberKey],
) -> Option<()> {
    let db = semantic_model.get_db();

    let mut index_map: HashMap<(LuaMemberKey, LuaType), Vec<TextRange>> = HashMap::new();

    for field in fields {
        let row_typ = semantic_model
            .infer_expr(field.get_value_expr().clone()?)
            .ok()?;
        let member_infos = semantic_model.get_member_infos(&row_typ)?;

        for member_info in member_infos {
            if !keys.contains(&member_info.key) {
                continue;
            }

            let range = match member_info.property_owner_id {
                Some(LuaSemanticDeclId::Member(member_id)) => member_id.get_syntax_id().get_range(),
                _ => continue,
            };

            index_map
                .entry((member_info.key.clone(), member_info.typ.clone()))
                .or_default()
                .push(range);
        }
    }

    for ((key, value), ranges) in index_map {
        if ranges.len() <= 1 {
            continue;
        }

        let name = if keys.len() > 1 {
            format!(
                "{}={}",
                key.to_path(),
                humanize_type(db, &value, RenderLevel::Simple)
            )
        } else {
            humanize_type(db, &value, RenderLevel::Simple)
        };

        for range in ranges {
            context.add_diagnostic(
                DiagnosticCode::DuplicatePrimaryKey,
                range,
                t!("Duplicate primary key `%{name}`", name = name).to_string(),
                None,
            );
        }
    }

    Some(())
}

fn check_duplicate_primary_key_union(
    context: &mut DiagnosticContext,
    semantic_model: &SemanticModel,
    fields: &[emmylua_parser::LuaTableField],
    keys: &[LuaMemberKey],
) -> Option<()> {
    let db = semantic_model.get_db();

    let mut index_map: HashMap<Vec<LuaType>, Vec<TextRange>> = HashMap::new();

    for field in fields {
        let row_typ = semantic_model
            .infer_expr(field.get_value_expr().clone()?)
            .ok()?;
        let member_infos = semantic_model.get_member_infos(&row_typ)?;

        let mut values = HashMap::new();
        for member_info in member_infos {
            if !keys.contains(&member_info.key) {
                continue;
            }

            let range = match member_info.property_owner_id {
                Some(LuaSemanticDeclId::Member(member_id)) => member_id.get_syntax_id().get_range(),
                _ => continue,
            };

            values.insert(member_info.key.clone(), (member_info.typ.clone(), range));
        }

        if !keys.iter().all(|k| values.contains_key(k)) {
            continue;
        }

        let value_tuple = keys
            .iter()
            .filter_map(|k| values.get(k).map(|(ty, _)| ty.clone()))
            .collect::<Vec<_>>();
        let ranges = keys
            .iter()
            .filter_map(|k| values.get(k).map(|(_, range)| *range))
            .collect::<Vec<_>>();

        let entry = index_map.entry(value_tuple).or_default();
        entry.extend(ranges);
    }

    for (value_tuple, ranges) in index_map {
        if ranges.len() <= keys.len() {
            continue;
        }

        let mut name = String::new();
        name.push('[');
        for (idx, (key, value)) in keys.iter().zip(value_tuple.iter()).enumerate() {
            if idx > 0 {
                name.push_str(", ");
            }
            name.push_str(&key.to_path());
            name.push('=');
            name.push_str(&humanize_type(db, value, RenderLevel::Simple));
        }
        name.push(']');

        for range in ranges {
            context.add_diagnostic(
                DiagnosticCode::DuplicatePrimaryKey,
                range,
                t!("Duplicate primary key `%{name}`", name = name).to_string(),
                None,
            );
        }
    }

    Some(())
}
