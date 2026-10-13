use std::ops::ControlFlow;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use super::{RowView, connection, scan};
use crate::anchor::Mark;
use crate::chain::{Fields, JournalId, Link};
use crate::error::JournalError;
use crate::model::{Seq, Timestamp};

const INSERT: &str = "
    INSERT INTO events (seq, at, actor, kind, call, action, resource, details, link)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
";

const PRUNE_CHUNK: i64 = 2_000;

pub(crate) struct WriteDb {
    connection: Connection,
}

impl WriteDb {
    pub(crate) fn create(path: &Path, journal: JournalId) -> Result<Self, JournalError> {
        let connection = connection::create(path, journal)?;
        Ok(Self { connection })
    }

    pub(crate) fn open(path: &Path) -> Result<Self, JournalError> {
        let connection = connection::open_writable(path)?;
        Ok(Self { connection })
    }

    pub(crate) fn journal_id(&self) -> Result<JournalId, JournalError> {
        connection::journal_id(&self.connection)
    }

    pub(crate) fn tail(&self) -> Result<Option<(Seq, Timestamp)>, JournalError> {
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

        let seq = Seq::from_sql(seq).ok_or(JournalError::Corrupt {
            seq: Seq::ORIGIN,
            reason: "negative sequence number",
        })?;
        Ok(Some((seq, Timestamp::from_millis(at))))
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

    pub(crate) fn scan<B>(
        &self,
        after: Seq,
        visit: impl FnMut(&RowView<'_>) -> ControlFlow<B>,
    ) -> Result<Option<B>, JournalError> {
        scan::scan(&self.connection, after, visit)
    }

    pub(crate) fn insert<'a>(
        &mut self,
        rows: impl IntoIterator<Item = (Fields<'a>, Link)>,
    ) -> Result<(), JournalError> {
        let transaction = self.connection.transaction()?;

        {
            let mut statement = transaction.prepare_cached(INSERT)?;

            for (fields, link) in rows {
                statement.execute(params![
                    fields.seq.to_sql(),
                    fields.at.as_millis(),
                    fields.actor,
                    fields.kind,
                    fields.call.map(Seq::to_sql),
                    fields.action,
                    fields.resource,
                    fields.details,
                    link.as_bytes().as_slice(),
                ])?;
            }
        }

        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn last_before(
        &self,
        at: Timestamp,
        limit: Seq,
    ) -> Result<Option<Mark>, JournalError> {
        let found: Option<(i64, Vec<u8>)> = self
            .connection
            .query_row(
                "SELECT seq, link FROM events WHERE at < ?1 AND seq <= ?2 ORDER BY seq DESC LIMIT 1",
                (at.as_millis(), limit.to_sql()),
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        let Some((seq, link)) = found else {
            return Ok(None);
        };

        let seq = Seq::from_sql(seq).ok_or(JournalError::Corrupt {
            seq: Seq::ORIGIN,
            reason: "negative sequence number",
        })?;
        let link = Link::from_slice(&link).ok_or(JournalError::Corrupt {
            seq,
            reason: "link has the wrong length",
        })?;

        Ok(Some(Mark::new(seq, link)))
    }

    pub(crate) fn delete_through(&self, seq: Seq) -> Result<u64, JournalError> {
        let mut removed = 0;

        loop {
            let chunk = self.connection.execute(
                "DELETE FROM events WHERE seq IN (SELECT seq FROM events WHERE seq <= ?1 ORDER BY seq LIMIT ?2)",
                (seq.to_sql(), PRUNE_CHUNK),
            )?;

            let chunk = i64::try_from(chunk).unwrap_or(i64::MAX);
            removed += chunk.unsigned_abs();

            if chunk < PRUNE_CHUNK {
                break;
            }
        }

        self.connection.execute_batch("PRAGMA incremental_vacuum")?;
        Ok(removed)
    }
}
