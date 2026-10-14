use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

use super::Access;
use crate::anchors::Mark;
use crate::error::JournalError;
use crate::journal::JournalId;
use crate::seq::Seq;
use crate::store::schema::Schema;
use crate::store::{Entry, RowView, Store};
use crate::timestamp::Timestamp;

const INSERT: &str = "
    INSERT INTO events (seq, at, actor, kind, call, action, resource, details, link)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
";

const PRUNE_CHUNK: i64 = 2_000;
const PENDING_SUFFIX: &str = ".pending";
const ROLLBACK_SUFFIX: &str = "-journal";

pub(crate) struct Writable;

impl Access for Writable {
    const FLAGS: OpenFlags =
        OpenFlags::SQLITE_OPEN_READ_WRITE.union(OpenFlags::SQLITE_OPEN_NO_MUTEX);

    fn prepare(connection: &Connection) -> Result<(), JournalError> {
        let mode: String =
            connection.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;

        if !mode.eq_ignore_ascii_case("wal") {
            return Err(JournalError::NotAJournal(
                "write-ahead logging is unavailable",
            ));
        }

        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.pragma_update(None, "fullfsync", true)?;
        connection.pragma_update(None, "checkpoint_fullfsync", true)?;

        Ok(())
    }
}

impl Store<Writable> {
    pub(crate) fn create(path: &Path, journal: JournalId) -> Result<Self, JournalError> {
        let pending = Self::sibling(path, PENDING_SUFFIX);
        Self::remove_stale(&pending)?;

        Schema::build(&pending, journal)?;
        fs::rename(&pending, path)?;

        Self::open(path, journal)
    }

    pub(crate) fn insert<'a>(
        &mut self,
        entries: impl IntoIterator<Item = Entry<'a>>,
    ) -> Result<(), JournalError> {
        let transaction = self.connection.transaction()?;

        {
            let mut statement = transaction.prepare_cached(INSERT)?;

            for Entry { fields, link } in entries {
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

        let seq = RowView::seq(seq)?;
        let link = RowView::link(seq, &link)?;

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

    fn remove_stale(pending: &Path) -> Result<(), JournalError> {
        for path in [pending.to_owned(), Self::sibling(pending, ROLLBACK_SUFFIX)] {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }

        Ok(())
    }

    fn sibling(path: &Path, suffix: &str) -> PathBuf {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);

        PathBuf::from(name)
    }
}
