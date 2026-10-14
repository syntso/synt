use serde::{Deserialize, Serialize};

use crate::ExtensionError;
use crate::manifest::checks::{check_local_id, check_text};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusItem {
    pub id: String,
    pub title: String,
    pub view: String,
}

impl StatusItem {
    pub(crate) fn check(&self) -> Result<(), ExtensionError> {
        check_local_id("status id", &self.id)?;
        check_text("status title", &self.title)?;
        check_local_id("status view", &self.view)
    }
}
