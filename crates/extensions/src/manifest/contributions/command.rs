use serde::{Deserialize, Serialize};
use synt_proto::OneLine;

use crate::manifest::ActionName;

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
