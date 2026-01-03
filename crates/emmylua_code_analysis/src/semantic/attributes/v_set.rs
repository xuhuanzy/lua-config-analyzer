use std::{collections::BTreeSet, fmt};

use crate::{LuaAttributeUse, LuaCommonProperty, LuaType};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SetValue {
    Int(i64),
    String(String),
}

impl fmt::Display for SetValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SetValue::Int(i) => write!(f, "{}", i),
            SetValue::String(s) => write!(f, "\"{}\"", s),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetSpec {
    values: BTreeSet<SetValue>,
}

impl SetSpec {
    pub fn new(values: BTreeSet<SetValue>) -> Result<Self, SetParseError> {
        if values.is_empty() {
            return Err(SetParseError::new("values must not be empty"));
        }
        Ok(Self { values })
    }

    pub fn contains(&self, value: &SetValue) -> bool {
        self.values.contains(value)
    }

    pub fn values(&self) -> &BTreeSet<SetValue> {
        &self.values
    }
}

impl fmt::Display for SetSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("{")?;
        for (idx, value) in self.values.iter().enumerate() {
            if idx > 0 {
                f.write_str(", ")?;
            }
            value.fmt(f)?;
        }
        f.write_str("}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetParseError {
    pub message: String,
}

impl SetParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SetParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SetParseError {}

/// Luban set validator attribute.
pub struct VSetAttribute<'a> {
    inner: &'a LuaAttributeUse,
}

impl<'a> VSetAttribute<'a> {
    pub const NAME: &'static str = "v.set";

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

    pub fn parse(&self) -> Result<SetSpec, SetParseError> {
        if self.inner.args.len() != 1 {
            return Err(SetParseError::new("v.set expects exactly one parameter"));
        }

        let Some(first) = self
            .inner
            .get_param_by_name("values")
            .or_else(|| self.inner.args.first().and_then(|(_, t)| t.as_ref()))
        else {
            return Err(SetParseError::new("missing values parameter"));
        };

        parse_set_spec_type(first)
    }
}

pub(crate) fn parse_set_spec_type(values_ty: &LuaType) -> Result<SetSpec, SetParseError> {
    let values_ty = values_ty.strip_attributed();

    let LuaType::Tuple(tuple) = values_ty else {
        return Err(SetParseError::new(
            "values parameter must be a tuple literal like [1, 2]",
        ));
    };

    let mut values = BTreeSet::new();
    for element in tuple.get_types() {
        collect_literal_value_from_type(element, &mut values)?;
    }
    SetSpec::new(values)
}

fn collect_literal_value_from_type(
    ty: &LuaType,
    out: &mut BTreeSet<SetValue>,
) -> Result<(), SetParseError> {
    match ty.strip_attributed() {
        LuaType::IntegerConst(i) | LuaType::DocIntegerConst(i) => {
            out.insert(SetValue::Int(*i));
            Ok(())
        }
        LuaType::StringConst(s) | LuaType::DocStringConst(s) => {
            out.insert(SetValue::String(s.as_ref().to_string()));
            Ok(())
        }
        _ => Err(SetParseError::new(
            "values tuple element must be a literal int or string",
        )),
    }
}
