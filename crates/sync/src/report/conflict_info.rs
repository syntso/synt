use serde::Serialize;

use crate::id::RevisionId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConflictInfo {
    pub path: String,
    pub merged: Option<RevisionId>,
    pub candidates: Vec<RevisionId>,
    pub fields: Vec<String>,
}
