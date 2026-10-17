use schemars::JsonSchema;
use serde::Deserialize;

use crate::CoreError;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(try_from = "String")]
pub(crate) struct RemotePath(String);

impl RemotePath {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

impl TryFrom<String> for RemotePath {
    type Error = CoreError;

    fn try_from(path: String) -> Result<Self, CoreError> {
        if path.is_empty() {
            return Err(CoreError::EmptyPath);
        }

        if path.contains('\0') {
            return Err(CoreError::NulInPath);
        }

        Ok(Self(path))
    }
}
