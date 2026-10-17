use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Header {
    name: String,
    value: String,
}

impl Header {
    pub(crate) fn new(name: String, value: String) -> Self {
        Self { name, value }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn value(&self) -> &str {
        &self.value
    }
}
