use rusqlite::Row;

use super::row::{FromRow, TypedRow};
use crate::document::DocumentPath;
use crate::id::{DocumentId, RevisionId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredDocument {
    pub(crate) document: DocumentId,
    pub(crate) path: DocumentPath,
    pub(crate) on_disk: Option<RevisionId>,
}

impl FromRow for StoredDocument {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            document: row.parsed("document")?,
            path: row.parsed("path")?,
            on_disk: row.parsed_or_null("on_disk")?,
        })
    }
}
