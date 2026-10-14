use serde::{Deserialize, Serialize};

use crate::ExtensionError;
use crate::manifest::checks::{check_text, refused};

const RESOURCE_KINDS: [&str; 2] = ["host", "tunnel"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuItem {
    pub title: String,
    pub action: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<String>,
}

impl MenuItem {
    pub(crate) fn check(&self) -> Result<(), ExtensionError> {
        check_text("menu title", &self.title)?;

        match self
            .kinds
            .iter()
            .find(|kind| !RESOURCE_KINDS.contains(&kind.as_str()))
        {
            Some(kind) => Err(refused(format!(
                "menu {:?} names the resource kind {kind:?}; kinds are {}",
                self.title,
                RESOURCE_KINDS.join(" and ")
            ))),
            None => Ok(()),
        }
    }
}
