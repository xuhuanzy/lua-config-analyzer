use crate::{LuaAttributeUse, LuaCommonProperty, LuaType};

/// 配置表索引模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigTableIndexMode {
    /// 独立索引
    Solo,
    /// 联合索引
    #[default]
    Union,
}

/// 定义配置表的索引字段列表
pub struct TIndexAttribute<'a> {
    inner: &'a LuaAttributeUse,
}

impl<'a> TIndexAttribute<'a> {
    pub const NAME: &'static str = "t.index";

    /// 从 Property 中查找此特性
    pub fn find_in(property: &'a LuaCommonProperty) -> Option<Self> {
        property
            .find_attribute_use(Self::NAME)
            .map(|inner| Self { inner })
    }

    /// 获取 indexs 参数
    pub fn get_indexs(&self) -> Option<&LuaType> {
        self.inner
            .get_param_by_name("indexs")
            .or_else(|| self.inner.args.first().and_then(|(_, t)| t.as_ref()))
    }

    /// 获取 mode 参数, 返回解析后的枚举
    pub fn get_mode(&self) -> ConfigTableIndexMode {
        let Some(mode_ty) = self.inner.get_param_by_name("mode") else {
            return ConfigTableIndexMode::Union;
        };

        match mode_ty {
            LuaType::DocStringConst(s) | LuaType::StringConst(s) if s.as_ref() == "solo" => {
                ConfigTableIndexMode::Solo
            }
            _ => ConfigTableIndexMode::Union,
        }
    }
}

/// 配置表模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigTableMode {
    #[default]
    Map,
    List,
    Singleton,
}

/// 定义配置表模式
pub struct TModeAttribute<'a> {
    inner: &'a LuaAttributeUse,
}

impl<'a> TModeAttribute<'a> {
    pub const NAME: &'static str = "t.mode";

    pub fn find_in(property: &'a LuaCommonProperty) -> Option<Self> {
        property
            .find_attribute_use(Self::NAME)
            .map(|inner| Self { inner })
    }

    pub fn get_mode(&self) -> ConfigTableMode {
        let mode_ty = self
            .inner
            .get_param_by_name("mode")
            .or_else(|| self.inner.args.first().and_then(|(_, t)| t.as_ref()));

        let Some(mode_ty) = mode_ty else {
            return ConfigTableMode::Map;
        };

        match mode_ty {
            LuaType::DocStringConst(s) | LuaType::StringConst(s) => match s.as_ref().as_str() {
                "list" => ConfigTableMode::List,
                "singleton" => ConfigTableMode::Singleton,
                _ => ConfigTableMode::Map,
            },
            _ => ConfigTableMode::Map,
        }
    }
}

/// 检查 list/array 内字段值唯一性
pub struct VIndexAttribute<'a> {
    inner: &'a LuaAttributeUse,
}

impl<'a> VIndexAttribute<'a> {
    pub const NAME: &'static str = "v.index";

    pub fn find_in(property: &'a LuaCommonProperty) -> Option<Self> {
        property
            .find_attribute_use(Self::NAME)
            .map(|inner| Self { inner })
    }

    pub fn find_all_in_uses(attribute_uses: &'a [LuaAttributeUse]) -> Vec<Self> {
        attribute_uses
            .iter()
            .filter(|attribute_use| attribute_use.id.get_name() == Self::NAME)
            .map(|inner| Self { inner })
            .collect()
    }

    pub fn get_key(&self) -> Option<&str> {
        let ty = self
            .inner
            .get_param_by_name("key")
            .or_else(|| self.inner.args.first().and_then(|(_, t)| t.as_ref()))?;

        match ty {
            LuaType::DocStringConst(s) | LuaType::StringConst(s) => Some(s.as_ref().as_str()),
            _ => None,
        }
    }
}

/// 检查字段值是否为配置表合法 key
pub struct VRefAttribute<'a> {
    inner: &'a LuaAttributeUse,
}

impl<'a> VRefAttribute<'a> {
    pub const NAME: &'static str = "v.ref";

    pub fn find_in(property: &'a LuaCommonProperty) -> Option<Self> {
        property
            .find_attribute_use(Self::NAME)
            .map(|inner| Self { inner })
    }

    pub fn find_in_uses(attribute_uses: &'a [LuaAttributeUse]) -> Option<Self> {
        attribute_uses
            .iter()
            .find(|attribute_use| attribute_use.id.get_name() == Self::NAME)
            .map(|inner| Self { inner })
    }

    pub fn get_table_name(&self) -> Option<&str> {
        let ty = self
            .inner
            .get_param_by_name("tableName")
            .or_else(|| self.inner.args.first().and_then(|(_, t)| t.as_ref()))?;

        match ty {
            LuaType::DocStringConst(s) | LuaType::StringConst(s) => Some(s.as_ref().as_str()),
            _ => None,
        }
    }

    pub fn get_field_name(&self) -> Option<&str> {
        let ty = self
            .inner
            .get_param_by_name("field")
            .or_else(|| self.inner.args.get(1).and_then(|(_, t)| t.as_ref()))?;

        match ty {
            LuaType::DocStringConst(s) | LuaType::StringConst(s) => Some(s.as_ref().as_str()),
            _ => None,
        }
    }
}

pub fn is_flags_attribute(property: &LuaCommonProperty) -> bool {
    property.find_attribute_use("flags").is_some()
}
