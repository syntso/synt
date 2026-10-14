use serde::{Deserialize, Serialize};

use crate::ExtensionError;
use crate::manifest::checks::{check_local_id, check_text, is_asset_path, refused};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Panel {
    pub id: String,
    pub title: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
}

impl Panel {
    pub(crate) fn check(&self) -> Result<(), ExtensionError> {
        check_local_id("panel id", &self.id)?;
        check_text("panel title", &self.title)?;

        match (&self.view, &self.entry) {
            (Some(view), None) => check_local_id("panel view", view),
            (None, Some(entry)) if is_asset_path(entry) => Ok(()),
            (None, Some(entry)) => Err(refused(format!(
                "panel {} has the entry {entry:?}, which is not a plain path inside the bundle",
                self.id
            ))),
            _ => Err(refused(format!(
                "panel {} needs either a native view or a web entry, not both",
                self.id
            ))),
        }
    }

    pub fn is_web(&self) -> bool {
        self.entry.is_some()
    }
}
