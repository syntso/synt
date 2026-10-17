use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::CoreError;

const LONGEST_NAME: usize = 64;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct ResourceName(String);

impl ResourceName {
    pub(crate) fn parse(name: &str) -> Result<Self, CoreError> {
        let allowed =
            |character: char| character.is_ascii_alphanumeric() || "-_.".contains(character);

        if name.is_empty() || name.len() > LONGEST_NAME {
            return Err(CoreError::NameLength(LONGEST_NAME));
        }

        if name.starts_with('.') || !name.chars().all(allowed) {
            return Err(CoreError::InvalidName(name.to_owned()));
        }

        Ok(Self(name.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ResourceName {
    type Error = CoreError;

    fn try_from(name: String) -> Result<Self, CoreError> {
        Self::parse(&name)
    }
}

impl From<ResourceName> for String {
    fn from(name: ResourceName) -> Self {
        name.0
    }
}

impl fmt::Display for ResourceName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
