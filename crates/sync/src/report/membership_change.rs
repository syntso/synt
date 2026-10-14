use serde::Serialize;

use crate::id::{Generation, MemberId, ProjectId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MembershipChange {
    pub project: ProjectId,
    pub member: MemberId,
    pub generation: Generation,
}
