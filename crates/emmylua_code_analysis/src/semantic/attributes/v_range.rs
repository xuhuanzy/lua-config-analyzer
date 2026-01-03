use std::fmt;

use crate::{LuaAttributeUse, LuaCommonProperty, LuaType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeEnd {
    Open,
    Closed,
}

impl RangeEnd {
    fn from_left_bracket(ch: char) -> Option<Self> {
        match ch {
            '(' => Some(Self::Open),
            '[' => Some(Self::Closed),
            _ => None,
        }
    }

    fn from_right_bracket(ch: char) -> Option<Self> {
        match ch {
            ')' => Some(Self::Open),
            ']' => Some(Self::Closed),
            _ => None,
        }
    }

    fn left_bracket(self) -> char {
        match self {
            Self::Open => '(',
            Self::Closed => '[',
        }
    }

    fn right_bracket(self) -> char {
        match self {
            Self::Open => ')',
            Self::Closed => ']',
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RangeSpec {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub min_end: RangeEnd,
    pub max_end: RangeEnd,
}

impl RangeSpec {
    pub fn exact(value: f64) -> Self {
        Self {
            min: Some(value),
            max: Some(value),
            min_end: RangeEnd::Closed,
            max_end: RangeEnd::Closed,
        }
    }

    pub fn closed(min: f64, max: f64) -> Result<Self, RangeParseError> {
        if min > max {
            return Err(RangeParseError::new("min must be <= max"));
        }

        Ok(Self {
            min: Some(min),
            max: Some(max),
            min_end: RangeEnd::Closed,
            max_end: RangeEnd::Closed,
        })
    }

    pub fn contains(&self, value: f64) -> bool {
        if let Some(min) = self.min {
            match self.min_end {
                RangeEnd::Closed => {
                    if value < min {
                        return false;
                    }
                }
                RangeEnd::Open => {
                    if value <= min {
                        return false;
                    }
                }
            }
        }

        if let Some(max) = self.max {
            match self.max_end {
                RangeEnd::Closed => {
                    if value > max {
                        return false;
                    }
                }
                RangeEnd::Open => {
                    if value >= max {
                        return false;
                    }
                }
            }
        }

        true
    }
}

impl fmt::Display for RangeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.min_end == RangeEnd::Closed
            && self.max_end == RangeEnd::Closed
            && self.min.is_some_and(|min| self.max == Some(min))
        {
            if let Some(min) = self.min {
                return write!(f, "{}", trim_float(min));
            }
        }

        write!(f, "{}", self.min_end.left_bracket())?;
        if let Some(min) = self.min {
            write!(f, "{}", trim_float(min))?;
        }
        write!(f, ",")?;
        if let Some(max) = self.max {
            write!(f, "{}", trim_float(max))?;
        }
        write!(f, "{}", self.max_end.right_bracket())
    }
}

fn trim_float(value: f64) -> String {
    let s = value.to_string();
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeParseError {
    pub message: String,
}

impl RangeParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RangeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RangeParseError {}

pub fn parse_range_spec(text: &str) -> Result<RangeSpec, RangeParseError> {
    let s = text.trim();
    if s.is_empty() {
        return Err(RangeParseError::new("empty range"));
    }

    let first = s
        .chars()
        .next()
        .ok_or_else(|| RangeParseError::new("empty range"))?;
    if first == '[' || first == '(' {
        let last = s
            .chars()
            .last()
            .ok_or_else(|| RangeParseError::new("empty range"))?;
        let Some(min_end) = RangeEnd::from_left_bracket(first) else {
            return Err(RangeParseError::new("invalid range left bracket"));
        };
        let Some(max_end) = RangeEnd::from_right_bracket(last) else {
            return Err(RangeParseError::new("invalid range right bracket"));
        };

        if s.len() < 2 {
            return Err(RangeParseError::new("invalid range"));
        }

        let inner = &s[1..s.len() - 1];
        let mut parts = inner.split(',');
        let Some(left) = parts.next() else {
            return Err(RangeParseError::new("invalid range"));
        };
        let Some(right) = parts.next() else {
            return Err(RangeParseError::new("range must contain a comma"));
        };
        if parts.next().is_some() {
            return Err(RangeParseError::new("range must contain exactly one comma"));
        }

        let left = left.trim();
        let right = right.trim();

        let min = if left.is_empty() {
            None
        } else {
            Some(
                left.parse::<f64>()
                    .map_err(|_| RangeParseError::new("invalid range min number"))?,
            )
        };
        let max = if right.is_empty() {
            None
        } else {
            Some(
                right
                    .parse::<f64>()
                    .map_err(|_| RangeParseError::new("invalid range max number"))?,
            )
        };

        if let (Some(min), Some(max)) = (min, max) {
            if min > max {
                return Err(RangeParseError::new("range min must be <= max"));
            }
        }

        Ok(RangeSpec {
            min,
            max,
            min_end,
            max_end,
        })
    } else {
        let value = s
            .parse::<f64>()
            .map_err(|_| RangeParseError::new("invalid range number"))?;
        Ok(RangeSpec::exact(value))
    }
}

/// Luban range validator attribute.
pub struct VRangeAttribute<'a> {
    inner: &'a LuaAttributeUse,
}

impl<'a> VRangeAttribute<'a> {
    pub const NAME: &'static str = "v.range";

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

    pub fn parse(&self) -> Result<RangeSpec, RangeParseError> {
        let Some(first) = self
            .inner
            .get_param_by_name("range")
            .or_else(|| self.inner.args.first().and_then(|(_, t)| t.as_ref()))
        else {
            return Err(RangeParseError::new("missing range parameter"));
        };

        match first {
            LuaType::IntegerConst(i) | LuaType::DocIntegerConst(i) => {
                Ok(RangeSpec::exact(*i as f64))
            }
            LuaType::FloatConst(f) => Ok(RangeSpec::exact(*f)),
            LuaType::DocStringConst(s) | LuaType::StringConst(s) => parse_range_spec(s.as_ref()),
            _ => Err(RangeParseError::new("invalid range parameter")),
        }
    }
}
