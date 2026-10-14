use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MessageParams {
    pub extension: String,
    pub panel: String,
    pub message: Value,
}
