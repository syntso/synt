use serde::Serialize;

use crate::id::{MemberId, ProjectId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Disabled {
    pub project: ProjectId,
    pub device: MemberId,
    pub left: bool,
}
