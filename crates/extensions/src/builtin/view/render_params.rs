use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RenderParams {
    pub extension: String,
    pub view: String,

    #[serde(default)]
    pub params: Value,
}
