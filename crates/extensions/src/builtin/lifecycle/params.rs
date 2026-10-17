use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct IdParams {
    id: String,
}

impl IdParams {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }
}
