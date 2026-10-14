use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetParams {
    pub extension: String,
    pub values: Value,
}
