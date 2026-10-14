use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PasswordInfo {
    pub name: String,
    pub reference: String,
    pub updated_at: u64,
}

impl PasswordInfo {
    pub(crate) fn summary(&self) -> Value {
        json!({ "name": self.name })
    }
}
