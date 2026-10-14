use serde::{Deserialize, Serialize};

use crate::ExtensionError;
use crate::manifest::checks::{check_local_id, check_text};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SidebarSection {
    pub id: String,
    pub title: String,
    pub view: String,
}

impl SidebarSection {
    pub(crate) fn check(&self) -> Result<(), ExtensionError> {
        check_local_id("sidebar id", &self.id)?;
        check_text("sidebar title", &self.title)?;
        check_local_id("sidebar view", &self.view)
    }
}
