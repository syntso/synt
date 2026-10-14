use serde::Serialize;
use synt_ssh::KeySummary;

use super::KeyName;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeyInfo {
    pub name: String,
    pub reference: String,
    pub kind: String,
    pub public_key: String,
    pub fingerprint: String,
    pub created_at: u64,
}

impl KeyInfo {
    pub(crate) fn new(name: &KeyName, summary: &KeySummary, created_at: u64) -> Self {
        Self {
            name: name.to_string(),
            reference: KeyName::reference(name.as_str()),
            kind: summary.kind.clone(),
            public_key: summary.public_key.clone(),
            fingerprint: summary.fingerprint.clone(),
            created_at,
        }
    }
}
