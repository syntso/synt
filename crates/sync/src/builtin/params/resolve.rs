use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResolveParams {
    pub project: String,
    pub path: String,
    pub revision: String,
}
