use serde::{Deserialize, Serialize};

use crate::manifest::{ActionName, OneLine};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    title: OneLine,
    action: ActionName,
}

impl Command {
    pub(crate) fn action(&self) -> &ActionName {
        &self.action
    }
}
