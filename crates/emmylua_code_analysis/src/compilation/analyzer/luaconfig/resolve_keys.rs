use crate::{
    ConfigTableIndexKeys, LuaMemberKey, LuaSemanticDeclId, LuaType,
    db_index::DbIndex,
    find_index_operations, is_sub_type_of,
    semantic::attributes::{ConfigTableIndexMode, TIndexAttribute},
};

/// 解析 ConfigTable 的索引键并缓存到 LuaConfigIndex
pub fn resolve_config_table_index(
    db: &mut DbIndex,
    file_id: crate::FileId,
    config_table_id: &crate::LuaTypeDeclId,
) {
    use crate::db_index::LuaMemberOwner;

    // 检查是否已经缓存
    if db.get_config_index().has_config_table_keys(config_table_id) {
        return;
    }

    // 获取 ConfigTable 的 [int] 成员 (Bean 类型)
    let config_table_type = LuaType::Ref(config_table_id.clone());
    let Some(members) = find_index_operations(db, &config_table_type) else {
        return;
    };
    let Some(int_member) = members
        .iter()
        .find(|m| matches!(m.key, LuaMemberKey::ExprType(LuaType::Integer)))
    else {
        return;
    };

    // 确定成员类型为 Bean
    let LuaType::Ref(bean_id) = &int_member.typ else {
        return;
    };

    // 检查是否是 Bean 的子类型 (递归检查父类)
    let bean_type_id = crate::LuaTypeDeclId::new(crate::BEAN_TYPE_NAME);
    let is_bean = is_sub_type_of(db, bean_id, &bean_type_id);
    if !is_bean {
        return;
    }

    // 获取 Bean 的成员列表
    let Some(bean_members_refs) = db
        .get_member_index()
        .get_members(&LuaMemberOwner::Type(bean_id.clone()))
    else {
        return;
    };
    let mut bean_members = bean_members_refs.to_vec();

    // 获取 ConfigTable 的 t.index 属性
    let property = db
        .get_property_index()
        .get_property(&LuaSemanticDeclId::TypeDecl(config_table_id.clone()));

    let index_keys = if let Some(property) = property {
        if let Some(index_attr) = TIndexAttribute::find_in(property) {
            // 从 t.index 属性解析索引键
            let (keys, mode) = resolve_index_keys_from_attr(&index_attr, &bean_members);
            if keys.is_empty() {
                // 回退到默认: 使用第一个成员作为索引
                bean_members.sort_by_key(|m| m.get_sort_key());
                let Some(first) = bean_members.first() else {
                    return;
                };
                let default_key = first.get_key().clone();
                ConfigTableIndexKeys::new(vec![default_key], ConfigTableIndexMode::Union)
            } else {
                ConfigTableIndexKeys::new(keys, mode)
            }
        } else {
            // 没有 t.index 属性, 使用第一个成员作为默认索引
            bean_members.sort_by_key(|m| m.get_sort_key());
            let Some(first) = bean_members.first() else {
                return;
            };
            let default_key = first.get_key().clone();
            ConfigTableIndexKeys::new(vec![default_key], ConfigTableIndexMode::Union)
        }
    } else {
        // 没有属性, 使用第一个成员作为默认索引
        bean_members.sort_by_key(|m| m.get_sort_key());
        let Some(first) = bean_members.first() else {
            return;
        };
        let default_key = first.get_key().clone();
        ConfigTableIndexKeys::new(vec![default_key], ConfigTableIndexMode::Union)
    };

    // 缓存解析结果
    if let Some(keys) = index_keys {
        db.get_config_index_mut()
            .add_config_table_keys(file_id, config_table_id.clone(), keys);
    }
}

/// 从 t.index 属性解析索引键
fn resolve_index_keys_from_attr(
    index_attr: &TIndexAttribute,
    bean_members: &[&crate::LuaMember],
) -> (Vec<LuaMemberKey>, ConfigTableIndexMode) {
    let mut keys: Vec<LuaMemberKey> = index_attr
        .get_indexs()
        .map(|ty| {
            collect_index_names_from_type(ty)
                .into_iter()
                .map(LuaMemberKey::Name)
                .collect()
        })
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

fn collect_index_names_from_type(ty: &LuaType) -> Vec<smol_str::SmolStr> {
    use std::ops::Deref;
    match ty {
        LuaType::DocStringConst(s) | LuaType::StringConst(s) => vec![s.deref().clone()],
        LuaType::Tuple(tuple) => tuple
            .get_types()
            .iter()
            .flat_map(collect_index_names_from_type)
            .collect(),
        LuaType::Union(union) => union
            .into_vec()
            .iter()
            .flat_map(collect_index_names_from_type)
            .collect(),
        _ => Vec::new(),
    }
}
