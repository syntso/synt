use serde::{Deserialize, Serialize};

use crate::canonical::Canonical;
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

    pub(crate) fn associated_data(&self) -> Vec<u8> {
        Canonical::new("synt.sync.sealed")
            .text(self.project.get())
            .text(self.document.get())
            .text(self.revision.get())
            .texts(self.parents.iter().map(RevisionId::get))
            .number(u64::from(self.generation.get()))
            .number(self.at)
            .finish()
    }
}
