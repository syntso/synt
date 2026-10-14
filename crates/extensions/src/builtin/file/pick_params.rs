use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PickParams {
    pub extension: String,
    pub path: String,
    pub access: String,
}
