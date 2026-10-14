use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SecretInfo {
    pub name: String,
    pub reference: String,
    pub bytes: u64,
    pub created_at: u64,
}

impl SecretInfo {
    pub(crate) fn summary(&self) -> Value {
        json!({ "name": self.name, "bytes": self.bytes })
    }
}
