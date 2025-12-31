use std::sync::LazyLock;

use crate::{LuaType, LuaTypeDeclId};

pub struct ConfigTable {
    name: &'static str,
    id: LazyLock<LuaTypeDeclId>,
}

impl ConfigTable {
    pub const fn new() -> Self {
        Self {
            name: "ConfigTable",
            id: LazyLock::new(|| LuaTypeDeclId::new("ConfigTable")),
        }
    }

    pub fn get_id(&self) -> &LuaTypeDeclId {
        &self.id
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn matches_type(&self, ty: &LuaType) -> bool {
        matches!(ty, LuaType::Ref(id) if id.get_name() == self.name)
    }

    pub fn matches_decl(&self, id: &LuaTypeDeclId) -> bool {
        id.get_name() == self.name
    }
}

pub static CONFIG_TABLE: ConfigTable = ConfigTable::new();

pub struct Bean {
    name: &'static str,
    id: LazyLock<LuaTypeDeclId>,
}

impl Bean {
    pub const fn new() -> Self {
        Self {
            name: "Bean",
            id: LazyLock::new(|| LuaTypeDeclId::new("Bean")),
        }
    }

    pub fn get_id(&self) -> &LuaTypeDeclId {
        &self.id
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn matches_type(&self, ty: &LuaType) -> bool {
        matches!(ty, LuaType::Ref(id) if id.get_name() == self.name)
    }

    pub fn matches_decl(&self, id: &LuaTypeDeclId) -> bool {
        id.get_name() == self.name
    }
}

pub static BEAN: Bean = Bean::new();
