use crate::document::DocumentPath;
use crate::id::{DocumentId, RevisionId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredDocument {
    pub(crate) document: DocumentId,
    pub(crate) path: DocumentPath,
    pub(crate) on_disk: Option<RevisionId>,
}
