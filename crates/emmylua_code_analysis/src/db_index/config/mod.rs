mod config_table_index_keys;

use std::collections::{HashMap, HashSet};

pub use config_table_index_keys::ConfigTableIndexKeys;

use crate::{FileId, LuaType, LuaTypeDeclId, db_index::traits::LuaIndex};

pub const CONFIG_TABLE_TYPE_NAME: &str = "ConfigTable";

pub const BEAN_TYPE_NAME: &str = "Bean";

/// 检测类型是否为 ConfigTable 引用
pub fn is_config_table_type(ty: &LuaType) -> bool {
    matches!(ty, LuaType::Ref(id) if id.get_name() == CONFIG_TABLE_TYPE_NAME)
}

/// 检测类型声明 ID 是否为 ConfigTable
pub fn is_config_table_decl(id: &LuaTypeDeclId) -> bool {
    id.get_name() == CONFIG_TABLE_TYPE_NAME
}

/// 检测类型是否为 Bean 引用
pub fn is_bean_type(ty: &LuaType) -> bool {
    matches!(ty, LuaType::Ref(id) if id.get_name() == BEAN_TYPE_NAME)
}

/// 检测类型声明 ID 是否为 Bean
pub fn is_bean_decl(id: &LuaTypeDeclId) -> bool {
    id.get_name() == BEAN_TYPE_NAME
}

#[derive(Debug)]
pub struct LuaConfigIndex {
    config_table_keys: HashMap<LuaTypeDeclId, ConfigTableIndexKeys>,
    in_file_types: HashMap<FileId, HashSet<LuaTypeDeclId>>,
}

impl Default for LuaConfigIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl LuaConfigIndex {
    pub fn new() -> Self {
        Self {
            config_table_keys: HashMap::new(),
            in_file_types: HashMap::new(),
        }
    }

    /// 添加 ConfigTable 的索引键缓存
    pub fn add_config_table_keys(
        &mut self,
        file_id: FileId,
        id: LuaTypeDeclId,
        keys: ConfigTableIndexKeys,
    ) {
        self.config_table_keys.insert(id.clone(), keys);
        self.in_file_types.entry(file_id).or_default().insert(id);
    }

    /// 获取 ConfigTable 的索引键缓存
    pub fn get_config_table_keys(&self, id: &LuaTypeDeclId) -> Option<&ConfigTableIndexKeys> {
        self.config_table_keys.get(id)
    }

    /// 检查是否存在指定 ConfigTable 的缓存
    pub fn has_config_table_keys(&self, id: &LuaTypeDeclId) -> bool {
        self.config_table_keys.contains_key(id)
    }
}

impl LuaIndex for LuaConfigIndex {
    fn remove(&mut self, file_id: FileId) {
        if let Some(type_ids) = self.in_file_types.remove(&file_id) {
            for type_id in type_ids {
                self.config_table_keys.remove(&type_id);
            }
        }
    }

    fn clear(&mut self) {
        self.config_table_keys.clear();
        self.in_file_types.clear();
    }
}
