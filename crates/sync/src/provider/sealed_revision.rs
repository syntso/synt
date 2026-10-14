use serde::{Deserialize, Serialize};

use crate::encoding::Base64;
use crate::id::{DocumentId, Generation, ProjectId, RevisionId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedRevision {
    pub project: ProjectId,
    pub document: DocumentId,
    pub revision: RevisionId,
    pub parents: Vec<RevisionId>,
    pub generation: Generation,
    pub at: u64,
    pub sealed: Base64,
}

impl SealedRevision {
    pub fn size(&self) -> usize {
        self.sealed.get().len()
    }
}
