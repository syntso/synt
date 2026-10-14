use std::fmt;

use serde::{Deserialize, Serialize};

use super::hex::{check_hex_id, hex_id};
use crate::SyncError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RevisionId(String);

impl RevisionId {
    pub fn parse(text: &str) -> Result<Self, SyncError> {
        check_hex_id("revision id", text)?;

        Ok(Self(text.to_owned()))
    }

    pub(crate) fn from_digest(digest: &[u8]) -> Self {
        Self(hex_id(digest))
    }

    pub fn get(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RevisionId {
    type Error = SyncError;

    fn try_from(text: String) -> Result<Self, SyncError> {
        check_hex_id("revision id", &text)?;

        Ok(Self(text))
    }
}

impl From<RevisionId> for String {
    fn from(id: RevisionId) -> Self {
        id.0
    }
}

impl fmt::Display for RevisionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
