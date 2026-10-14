use schemars::JsonSchema;
use serde::Deserialize;

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
    type Error = String;

    fn try_from(path: String) -> Result<Self, String> {
        if path.is_empty() {
            return Err("the path is empty".to_owned());
        }

        if path.contains('\0') {
            return Err("the path contains a NUL character".to_owned());
        }

        Ok(Self(path))
    }
}
