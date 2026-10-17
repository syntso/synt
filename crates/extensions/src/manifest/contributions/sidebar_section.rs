use serde::{Deserialize, Serialize};
use synt_proto::OneLine;

use crate::manifest::LocalId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SidebarSection {
    id: LocalId,
    title: OneLine,
    view: LocalId,
}

impl SidebarSection {
    pub(crate) fn id(&self) -> &LocalId {
        &self.id
    }

    pub(crate) fn view(&self) -> &LocalId {
        &self.view
    }
}
