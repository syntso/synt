use serde::{Deserialize, Serialize};

use crate::manifest::{LocalId, OneLine};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusItem {
    id: LocalId,
    title: OneLine,
    view: LocalId,
}

impl StatusItem {
    pub(crate) fn id(&self) -> &LocalId {
        &self.id
    }

    pub(crate) fn view(&self) -> &LocalId {
        &self.view
    }
}
