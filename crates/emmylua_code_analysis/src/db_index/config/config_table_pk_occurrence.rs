use rowan::TextRange;

use crate::{LuaMemberKey, LuaType, LuaTypeDeclId};

#[derive(Debug, Clone)]
pub enum ConfigTablePkOccurrence {
    Solo {
        config_table: LuaTypeDeclId,
        key: LuaMemberKey,
        value: LuaType,
        range: TextRange,
    },
    Union {
        config_table: LuaTypeDeclId,
        values: Vec<LuaType>,
        ranges: Vec<TextRange>,
    },
}

impl ConfigTablePkOccurrence {
    pub fn get_config_table(&self) -> &LuaTypeDeclId {
        match self {
            ConfigTablePkOccurrence::Solo { config_table, .. } => config_table,
            ConfigTablePkOccurrence::Union { config_table, .. } => config_table,
        }
    }
}
