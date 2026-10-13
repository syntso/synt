use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionCall {
    pub action: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    #[serde(default)]
    pub params: Value,
}

impl ActionCall {
    pub fn new(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            target: None,
            params: Value::Null,
        }
    }

    #[must_use]
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    #[must_use]
    pub fn params(mut self, params: Value) -> Self {
        self.params = params;
        self
    }
}
