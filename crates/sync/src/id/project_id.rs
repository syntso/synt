use std::fmt;

use serde::{Deserialize, Serialize};

use super::hex::check_hex_id;
use crate::SyncError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProjectId(String);

impl ProjectId {
    pub fn parse(text: &str) -> Result<Self, SyncError> {
        check_hex_id("project id", text)?;

        Ok(Self(text.to_owned()))
    }

    pub fn get(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ProjectId {
    type Error = SyncError;

    fn try_from(text: String) -> Result<Self, SyncError> {
        check_hex_id("project id", &text)?;

        Ok(Self(text))
    }
}

impl From<ProjectId> for String {
    fn from(id: ProjectId) -> Self {
        id.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
