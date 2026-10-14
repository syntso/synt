use serde::{Deserialize, Serialize};

use crate::ExtensionError;
use crate::manifest::checks::check_text;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub title: String,
    pub action: String,
}

impl Command {
    pub(crate) fn check(&self) -> Result<(), ExtensionError> {
        check_text("command title", &self.title)
    }
}
