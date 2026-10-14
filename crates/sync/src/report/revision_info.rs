use serde::Serialize;

use crate::id::{MemberId, RevisionId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RevisionInfo {
    pub revision: RevisionId,
    pub parents: Vec<RevisionId>,
    pub at: u64,
    pub author: Option<MemberId>,
    pub removed: bool,
    pub bytes: usize,
    pub conflicts: Vec<String>,
    pub head: bool,
    pub on_disk: bool,
}
