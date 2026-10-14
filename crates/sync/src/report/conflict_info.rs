use serde::Serialize;

use crate::document::DocumentPath;
use crate::id::RevisionId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConflictInfo {
    pub path: DocumentPath,
    pub merged: Option<RevisionId>,
    pub candidates: Vec<RevisionId>,
    pub fields: Vec<String>,
}
