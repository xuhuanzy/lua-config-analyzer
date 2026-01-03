use std::fmt;

use crate::{LuaAttributeUse, LuaCommonProperty, LuaType};

use super::{RangeParseError, RangeSpec, parse_range_spec};

#[derive(Debug, Clone, PartialEq)]
pub struct SizeSpec {
    inner: RangeSpec,
}

impl SizeSpec {
    pub fn exact(size: usize) -> Self {
        Self {
            inner: RangeSpec::exact(size as f64),
        }
    }

    pub fn contains_len(&self, len: usize) -> bool {
        self.inner.contains(len as f64)
    }

    pub fn as_range(&self) -> &RangeSpec {
        &self.inner
    }
}

impl fmt::Display for SizeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

fn validate_size_range(spec: RangeSpec) -> Result<SizeSpec, RangeParseError> {
    if let Some(min) = spec.min {
        if !min.is_finite() || min.fract() != 0.0 {
            return Err(RangeParseError::new("size min must be an integer"));
        }
        if min < 0.0 {
            return Err(RangeParseError::new("size min must be >= 0"));
        }
    }
    if let Some(max) = spec.max {
        if !max.is_finite() || max.fract() != 0.0 {
            return Err(RangeParseError::new("size max must be an integer"));
        }
        if max < 0.0 {
            return Err(RangeParseError::new("size max must be >= 0"));
        }
    }

    Ok(SizeSpec { inner: spec })
}

/// Luban size validator attribute.
pub struct VSizeAttribute<'a> {
    inner: &'a LuaAttributeUse,
}

impl<'a> VSizeAttribute<'a> {
    pub const NAME: &'static str = "v.size";

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

    pub fn find_all_in_uses(attribute_uses: &'a [LuaAttributeUse]) -> Vec<Self> {
        attribute_uses
            .iter()
            .filter(|attribute_use| attribute_use.id.get_name() == Self::NAME)
            .map(|inner| Self { inner })
            .collect()
    }

    pub fn parse(&self) -> Result<SizeSpec, RangeParseError> {
        if self.inner.args.len() != 1 {
            return Err(RangeParseError::new("v.size expects exactly one parameter"));
        }

        let Some(first) = self
            .inner
            .get_param_by_name("size")
            .or_else(|| self.inner.args.first().and_then(|(_, t)| t.as_ref()))
        else {
            return Err(RangeParseError::new("missing size parameter"));
        };

        match first {
            LuaType::IntegerConst(i) | LuaType::DocIntegerConst(i) => {
                if *i < 0 {
                    return Err(RangeParseError::new("size must be >= 0"));
                }
                Ok(SizeSpec::exact(*i as usize))
            }
            LuaType::DocStringConst(s) | LuaType::StringConst(s) => {
                let range = parse_range_spec(s.as_ref())?;
                validate_size_range(range)
            }
            _ => Err(RangeParseError::new("invalid size parameter")),
        }
    }
}
