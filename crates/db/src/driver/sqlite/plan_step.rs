use rusqlite::Row;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct PlanStep {
    id: i64,
    parent: i64,
    detail: String,
}

impl TryFrom<&Row<'_>> for PlanStep {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            parent: row.get("parent")?,
            detail: row.get("detail")?,
        })
    }
}
