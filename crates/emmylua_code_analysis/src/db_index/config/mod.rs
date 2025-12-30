mod config_table_index_keys;
mod config_table_pk_occurrence;

use std::collections::{HashMap, HashSet};

pub use config_table_index_keys::ConfigTableIndexKeys;
pub use config_table_pk_occurrence::ConfigTablePkOccurrence;

use crate::{
    FileId, LuaType, LuaTypeDeclId, db_index::traits::LuaIndex,
    semantic::attributes::ConfigTableMode,
};

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
    config_table_modes: HashMap<LuaTypeDeclId, ConfigTableMode>,
    in_file_types: HashMap<FileId, HashSet<LuaTypeDeclId>>,
    config_table_pk_occurrences: HashMap<FileId, Vec<ConfigTablePkOccurrence>>,
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
            config_table_modes: HashMap::new(),
            in_file_types: HashMap::new(),
            config_table_pk_occurrences: HashMap::new(),
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

    /// 添加 ConfigTable 的 mode 缓存
    pub fn add_config_table_mode(
        &mut self,
        file_id: FileId,
        id: LuaTypeDeclId,
        mode: ConfigTableMode,
    ) {
        self.config_table_modes.insert(id.clone(), mode);
        self.in_file_types.entry(file_id).or_default().insert(id);
    }

    /// 获取 ConfigTable 的索引键缓存
    pub fn get_config_table_keys(&self, id: &LuaTypeDeclId) -> Option<&ConfigTableIndexKeys> {
        self.config_table_keys.get(id)
    }

    /// 获取 ConfigTable 的 mode 缓存
    pub fn get_config_table_mode(&self, id: &LuaTypeDeclId) -> ConfigTableMode {
        self.config_table_modes
            .get(id)
            .copied()
            .unwrap_or(ConfigTableMode::Map)
    }

    /// 检查是否存在指定 ConfigTable 的缓存
    pub fn has_config_table_keys(&self, id: &LuaTypeDeclId) -> bool {
        self.config_table_keys.contains_key(id)
    }

    pub fn has_config_table_mode(&self, id: &LuaTypeDeclId) -> bool {
        self.config_table_modes.contains_key(id)
    }

    pub fn get_config_table_pk_occurrences(
        &self,
        file_id: &FileId,
    ) -> Option<&Vec<ConfigTablePkOccurrence>> {
        self.config_table_pk_occurrences.get(file_id)
    }

    pub fn iter_config_table_pk_occurrences(
        &self,
    ) -> impl Iterator<Item = &ConfigTablePkOccurrence> {
        self.config_table_pk_occurrences
            .values()
            .flat_map(|v| v.iter())
    }

    pub fn add_config_table_pk_occurrences(
        &mut self,
        file_id: FileId,
        occurrences: Vec<ConfigTablePkOccurrence>,
    ) {
        self.config_table_pk_occurrences.remove(&file_id);
        if !occurrences.is_empty() {
            self.config_table_pk_occurrences
                .insert(file_id, occurrences);
        }
    }
}

impl LuaIndex for LuaConfigIndex {
    fn remove(&mut self, file_id: FileId) {
        self.config_table_pk_occurrences.remove(&file_id);
        if let Some(type_ids) = self.in_file_types.remove(&file_id) {
            for type_id in type_ids {
                self.config_table_keys.remove(&type_id);
                self.config_table_modes.remove(&type_id);
            }
        }
    }

    fn clear(&mut self) {
        self.config_table_keys.clear();
        self.config_table_modes.clear();
        self.in_file_types.clear();
        self.config_table_pk_occurrences.clear();
    }
}
