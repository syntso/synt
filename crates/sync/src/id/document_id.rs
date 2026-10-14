use std::fmt;

use serde::{Deserialize, Serialize};

use super::hex::{check_hex_id, hex_id};
use crate::SyncError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DocumentId(String);

impl DocumentId {
    pub fn parse(text: &str) -> Result<Self, SyncError> {
        check_hex_id("document id", text)?;

        Ok(Self(text.to_owned()))
    }

    pub(crate) fn from_digest(digest: &[u8]) -> Self {
        Self(hex_id(digest))
    }

    pub fn get(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for DocumentId {
    type Error = SyncError;

    fn try_from(text: String) -> Result<Self, SyncError> {
        check_hex_id("document id", &text)?;

        Ok(Self(text))
    }
}

impl From<DocumentId> for String {
    fn from(id: DocumentId) -> Self {
        id.0
    }
}

impl fmt::Display for DocumentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
