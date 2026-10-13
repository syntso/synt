use std::ops::ControlFlow;

use rusqlite::Connection;

use super::RowView;
use super::row::COLUMNS;
use crate::error::JournalError;
use crate::model::Seq;

pub(super) fn scan<B>(
    connection: &Connection,
    after: Seq,
    mut visit: impl FnMut(&RowView<'_>) -> ControlFlow<B>,
) -> Result<Option<B>, JournalError> {
    let mut statement = connection.prepare_cached(&format!(
        "SELECT {COLUMNS} FROM events WHERE seq > ?1 ORDER BY seq"
    ))?;
    let mut rows = statement.query([after.to_sql()])?;

    while let Some(row) = rows.next()? {
        let view = RowView::read(row)?;

        if let ControlFlow::Break(outcome) = visit(&view) {
            return Ok(Some(outcome));
        }
    }

    Ok(None)
}
