use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SecretHeader {
    name: String,
    secret: String,

    #[serde(default)]
    prefix: String,
}

impl SecretHeader {
    pub(crate) fn new(name: String, secret: String, prefix: String) -> Self {
        Self {
            name,
            secret,
            prefix,
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn secret(&self) -> &str {
        &self.secret
    }

    pub(crate) fn prefix(&self) -> &str {
        &self.prefix
    }
}
