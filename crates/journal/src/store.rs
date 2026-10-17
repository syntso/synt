mod access;
mod entry;
mod query;
mod row;
mod schema;
mod tail;

use std::marker::PhantomData;
use std::ops::ControlFlow;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params_from_iter};

use self::access::Access;
use self::query::Query;
use self::row::COLUMNS;
use self::schema::Schema;
use crate::chain::Link;
use crate::error::JournalError;
use crate::event::Event;
use crate::filter::Filter;
use crate::journal::JournalId;
use crate::seq::Seq;
use crate::timestamp::Timestamp;

pub(crate) use access::{ReadOnly, Writable};
pub(crate) use entry::Entry;
pub(crate) use row::RowView;
pub(crate) use tail::Tail;

pub(crate) struct Store<A> {
    connection: Connection,
    access: PhantomData<A>,
}

impl<A: Access> Store<A> {
    pub(crate) fn open(path: &Path, journal: JournalId) -> Result<Self, JournalError> {
        let connection = Connection::open_with_flags(path, A::FLAGS)?;

        A::prepare(&connection)?;
        Schema::check(&connection, journal)?;

        Ok(Self {
            connection,
            access: PhantomData,
        })
    }

    pub(crate) fn get(&self, seq: Seq) -> Result<Option<Event>, JournalError> {
        let sql = format!("SELECT {COLUMNS} FROM events WHERE seq = ?1");
        let mut statement = self.connection.prepare_cached(&sql)?;
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
        mut visit: impl FnMut(&RowView<'_>) -> ControlFlow<B>,
    ) -> Result<Option<B>, JournalError> {
        let sql = format!("SELECT {COLUMNS} FROM events WHERE seq > ?1 ORDER BY seq");
        let mut statement = self.connection.prepare_cached(&sql)?;
        let mut rows = statement.query([after.to_sql()])?;

        while let Some(row) = rows.next()? {
            let view = RowView::read(row)?;

            if let ControlFlow::Break(outcome) = visit(&view) {
                return Ok(Some(outcome));
            }
        }

        Ok(None)
    }

    pub(crate) fn tail(&self) -> Result<Option<Tail>, JournalError> {
        let tail = self
            .connection
            .query_row(
                "SELECT seq, at FROM events ORDER BY seq DESC LIMIT 1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;

        let Some((seq, at)) = tail else {
            return Ok(None);
        };

        Ok(Some(Tail {
            seq: RowView::seq(seq)?,
            at: Timestamp::from_millis(at),
        }))
    }

    pub(crate) fn link_at(&self, seq: Seq) -> Result<Option<Link>, JournalError> {
        let bytes: Option<Vec<u8>> = self
            .connection
            .query_row(
                "SELECT link FROM events WHERE seq = ?1",
                [seq.to_sql()],
                |row| row.get(0),
            )
            .optional()?;

        Ok(bytes.as_deref().and_then(Link::from_slice))
    }
}
