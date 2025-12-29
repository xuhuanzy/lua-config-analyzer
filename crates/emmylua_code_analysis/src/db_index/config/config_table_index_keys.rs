use crate::{LuaMemberKey, semantic::attributes::ConfigTableIndexMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigTableIndexKeys {
    /// 独立索引模式: 每个键独立作为主键
    Solo(Vec<LuaMemberKey>),
    /// 联合索引模式: 所有键组合成复合主键
    Union(Vec<LuaMemberKey>),
}

impl ConfigTableIndexKeys {
    /// 创建新的索引键配置
    pub fn new(keys: Vec<LuaMemberKey>, mode: ConfigTableIndexMode) -> Option<Self> {
        if keys.is_empty() {
            return None;
        }

        // 单个键时, 模式不影响结果, 统一使用 Solo
        if keys.len() == 1 {
            return Some(Self::Solo(keys));
        }

        Some(match mode {
            ConfigTableIndexMode::Solo => Self::Solo(keys),
            ConfigTableIndexMode::Union => Self::Union(keys),
        })
    }

    /// 获取所有索引键
    pub fn keys(&self) -> &[LuaMemberKey] {
        match self {
            Self::Solo(keys) | Self::Union(keys) => keys,
        }
    }

    /// 检查是否为独立索引模式
    pub fn is_solo(&self) -> bool {
        matches!(self, Self::Solo(_))
    }

    /// 检查是否为联合索引模式
    pub fn is_union(&self) -> bool {
        matches!(self, Self::Union(_))
    }
}
