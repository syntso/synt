use rusqlite::Row;

use super::row::{FromRow, TypedRow};
use crate::id::ProjectId;
use crate::provider::{Cursor, Place};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyncedProject {
    pub(crate) project: ProjectId,
    pub(crate) place: Place,
    pub(crate) cursor: Cursor,
}

impl FromRow for SyncedProject {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            project: row.parsed("project", ProjectId::parse)?,
            place: row.json("place")?,
            cursor: Cursor::new(row.get::<_, String>("cursor")?),
        })
    }
}
