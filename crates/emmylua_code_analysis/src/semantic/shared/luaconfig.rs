use std::sync::LazyLock;

use crate::{LuaMemberKey, LuaType, LuaTypeDeclId, find_index_operations, is_sub_type_of};

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

    /// 判断是否为 ConfigTable 的子类型
    pub fn is_config_table(&self, db: &crate::DbIndex, id: &LuaTypeDeclId) -> bool {
        is_sub_type_of(db, id, &self.get_id())
    }

    /// 获取 ConfigTable 绑定的 Bean
    pub fn get_bean_id(&self, db: &crate::DbIndex, id: &LuaTypeDeclId) -> Option<LuaTypeDeclId> {
        if !self.is_config_table(db, id) {
            return None;
        }

        let members = find_index_operations(db, &LuaType::Ref(id.clone()))?;
        let int_member = members
            .iter()
            .find(|m| matches!(m.key, LuaMemberKey::ExprType(LuaType::Integer)))?;

        let LuaType::Ref(bean_id) = &int_member.typ else {
            return None;
        };

        if !BEAN.is_bean(db, bean_id) {
            return None;
        }

        Some(bean_id.clone())
    }
}

/// 配置表基类. 所有配置表都必须继承自 ConfigTable.
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

    /// 判断是否为 Bean 的子类型
    pub fn is_bean(&self, db: &crate::DbIndex, id: &LuaTypeDeclId) -> bool {
        is_sub_type_of(db, id, &self.get_id())
    }
}

/// Bean 基类. 所有 Bean 都必须继承自 Bean.
pub static BEAN: Bean = Bean::new();
