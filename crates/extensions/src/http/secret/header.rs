use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SecretHeader {
    pub(crate) name: String,
    pub(crate) secret: String,

    #[serde(default)]
    pub(crate) prefix: String,
}
