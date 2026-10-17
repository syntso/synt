use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExtensionParams {
    extension: String,
}

impl ExtensionParams {
    pub(crate) fn extension(&self) -> &str {
        &self.extension
    }
}
