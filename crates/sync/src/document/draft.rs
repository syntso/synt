use super::DocumentPath;
use crate::canonical::Canonical;
use crate::id::{DocumentId, MemberId, RevisionId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Draft {
    pub(crate) path: DocumentPath,
    pub(crate) parents: Vec<RevisionId>,
    pub(crate) at: u64,
    pub(crate) body: Option<Vec<u8>>,
    pub(crate) conflicts: Vec<String>,
    pub(crate) author: Option<MemberId>,
}

impl Draft {
    pub(crate) fn content(&self, document: &DocumentId) -> Vec<u8> {
        Canonical::new("synt.sync.revision")
            .text(document.get())
            .texts(self.parents.iter().map(RevisionId::get))
            .number(self.at)
            .text(self.path.get())
            .text(&self.path.kind().to_string())
            .optional(self.body.as_deref())
            .texts(self.conflicts.iter().map(String::as_str))
            .optional(self.author.as_ref().map(|author| author.get().as_bytes()))
            .finish()
    }
}
