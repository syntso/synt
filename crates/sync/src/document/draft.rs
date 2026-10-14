use super::DocumentPath;
use crate::id::{MemberId, RevisionId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Draft {
    pub(crate) path: DocumentPath,
    pub(crate) parents: Vec<RevisionId>,
    pub(crate) at: u64,
    pub(crate) body: Option<Vec<u8>>,
    pub(crate) conflicts: Vec<String>,
    pub(crate) author: Option<MemberId>,
}
