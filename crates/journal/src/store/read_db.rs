use std::ops::ControlFlow;
use std::path::Path;

use rusqlite::{Connection, params_from_iter};

use super::query::Query;
use super::row::COLUMNS;
use super::{RowView, connection, scan};
use crate::chain::JournalId;
use crate::error::JournalError;
use crate::filter::Filter;
use crate::model::{Event, Seq};

pub(crate) struct ReadDb {
    connection: Connection,
}

impl ReadDb {
    pub(crate) fn open(path: &Path, journal: JournalId) -> Result<Self, JournalError> {
        let connection = connection::open_read_only(path)?;
        connection::check_id(&connection, journal)?;

        Ok(Self { connection })
    }

    pub(crate) fn get(&self, seq: Seq) -> Result<Option<Event>, JournalError> {
        let mut statement = self
            .connection
            .prepare_cached(&format!("SELECT {COLUMNS} FROM events WHERE seq = ?1"))?;
        let mut rows = statement.query([seq.to_sql()])?;

        let Some(row) = rows.next()? else {
            return Ok(None);
        };

        RowView::read(row)?.to_event().map(Some)
    }

    pub(crate) fn page(
        &self,
        filter: &Filter,
        after: Seq,
        limit: u32,
    ) -> Result<Vec<Event>, JournalError> {
        let query = Query::page(filter, after, limit);

        let mut statement = self.connection.prepare_cached(&query.sql)?;
        let mut rows = statement.query(params_from_iter(query.params))?;
        let mut events = Vec::new();

        while let Some(row) = rows.next()? {
            events.push(RowView::read(row)?.to_event()?);
        }

        Ok(events)
    }

    pub(crate) fn scan<B>(
        &self,
        after: Seq,
        visit: impl FnMut(&RowView<'_>) -> ControlFlow<B>,
    ) -> Result<Option<B>, JournalError> {
        scan::scan(&self.connection, after, visit)
    }
}
