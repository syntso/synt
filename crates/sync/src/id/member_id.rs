use std::fmt;

use serde::{Deserialize, Serialize};

use super::hex::{check_hex_id, hex_id};
use crate::SyncError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MemberId(String);

impl MemberId {
    pub fn parse(text: &str) -> Result<Self, SyncError> {
        check_hex_id("member id", text)?;

        Ok(Self(text.to_owned()))
    }

    pub(crate) fn from_digest(digest: &[u8]) -> Self {
        Self(hex_id(digest))
    }

    pub fn get(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for MemberId {
    type Error = SyncError;

    fn try_from(text: String) -> Result<Self, SyncError> {
        check_hex_id("member id", &text)?;

        Ok(Self(text))
    }
}

impl From<MemberId> for String {
    fn from(id: MemberId) -> Self {
        id.0
    }
}

impl fmt::Display for MemberId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
