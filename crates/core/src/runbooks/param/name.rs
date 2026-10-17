use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::CoreError;

const LONGEST: usize = 64;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(try_from = "String", into = "String")]
pub struct ParamName(String);

impl ParamName {
    pub(crate) fn new(name: impl Into<String>) -> Result<Self, CoreError> {
        let name = name.into();

        if !Self::fits(&name) {
            return Err(CoreError::BadParamName(name));
        }

        Ok(Self(name))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    fn fits(name: &str) -> bool {
        let mut characters = name.chars();

        let Some(first) = characters.next() else {
            return false;
        };

        let starts_well = first.is_ascii_alphabetic() || first == '_';
        let continues_well =
            characters.all(|character| character.is_ascii_alphanumeric() || character == '_');

        starts_well && continues_well && name.len() <= LONGEST
    }
}

impl TryFrom<String> for ParamName {
    type Error = CoreError;

    fn try_from(name: String) -> Result<Self, CoreError> {
        Self::new(name)
    }
}

impl From<ParamName> for String {
    fn from(name: ParamName) -> Self {
        name.0
    }
}

impl Borrow<str> for ParamName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Display for ParamName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::ParamName;

    #[test]
    fn names_are_identifiers() {
        for good in ["host", "_tag", "service_2"] {
            assert!(ParamName::new(good).is_ok(), "{good}");
        }

        for bad in ["", "2nd", "dry-run", "a b", &"a".repeat(65)] {
            assert!(ParamName::new(bad).is_err(), "{bad}");
        }
    }
}
