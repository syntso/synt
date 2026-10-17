use serde::Serialize;
use serde_json::{Value, json};
use synt_journal::Timestamp;
use synt_platform::SecretName;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SecretInfo {
    pub name: SecretName,
    pub reference: String,
    pub bytes: u64,
    pub created_at: Timestamp,
}

impl SecretInfo {
    pub(crate) fn new(name: &SecretName, bytes: usize, created_at: Timestamp) -> Self {
        Self {
            name: name.clone(),
            reference: name.reference(),
            bytes: u64::try_from(bytes).unwrap_or(u64::MAX),
            created_at,
        }
    }

    pub(crate) fn summary(&self) -> Value {
        json!({ "name": self.name, "bytes": self.bytes })
    }
}
