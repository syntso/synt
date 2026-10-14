use serde::Serialize;

use crate::id::{MemberId, RevisionId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Applied {
    pub path: String,
    pub revision: RevisionId,
    pub author: Option<MemberId>,
    pub removed: bool,
    pub conflicts: usize,
}
