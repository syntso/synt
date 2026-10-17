use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::CoreError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct ContextName(String);

impl ContextName {
    pub(crate) fn parse(text: &str) -> Result<Self, CoreError> {
        if text.is_empty() || text.contains(char::is_control) {
            return Err(CoreError::NotAContext(text.to_owned()));
        }

        Ok(Self(text.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ContextName {
    type Error = CoreError;

    fn try_from(text: String) -> Result<Self, CoreError> {
        Self::parse(&text)
    }
}

impl From<ContextName> for String {
    fn from(context: ContextName) -> Self {
        context.0
    }
}
