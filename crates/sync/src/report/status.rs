use serde::Serialize;

use super::{Access, ConflictInfo, MemberInfo, RoundReport};
use crate::id::{Generation, MemberId, ProjectId};
use crate::provider::Place;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusReport {
    pub project: ProjectId,
    pub place: Place,
    pub device: MemberId,
    pub access: Access,
    pub generation: Option<Generation>,
    pub members: Vec<MemberInfo>,
    pub documents: usize,
    pub unpushed: usize,
    pub held: usize,
    pub conflicts: Vec<ConflictInfo>,
    pub last_round: Option<RoundReport>,
}
