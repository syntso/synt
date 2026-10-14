use serde::Serialize;
use serde_json::{Value, json};
use synt_journal::Timestamp;
use synt_ssh::KeySummary;

use super::KeyName;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeyInfo {
    pub name: KeyName,
    pub reference: String,
    pub kind: String,
    pub public_key: String,
    pub fingerprint: String,
    pub created_at: Timestamp,
}

impl KeyInfo {
    pub(crate) fn new(name: &KeyName, summary: &KeySummary, created_at: Timestamp) -> Self {
        Self {
            name: name.clone(),
            reference: name.reference(),
            kind: summary.kind.clone(),
            public_key: summary.public_key.clone(),
            fingerprint: summary.fingerprint.clone(),
            created_at,
        }
    }

    pub(crate) fn summary(&self) -> Value {
        json!({ "name": self.name, "kind": self.kind, "fingerprint": self.fingerprint })
    }
}
