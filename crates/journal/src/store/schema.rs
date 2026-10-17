use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::error::JournalError;
use crate::journal::JournalId;

const VERSION: i64 = 1;
const ID_KEY: &str = "journal_id";
const CREATE: &str = include_str!("../../sql/schema.sql");

pub(super) struct Schema;

impl Schema {
    pub(super) fn build(path: &Path, journal: JournalId) -> Result<(), JournalError> {
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let mut connection = Connection::open_with_flags(path, flags)?;

        connection.pragma_update(None, "auto_vacuum", "INCREMENTAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.pragma_update(None, "fullfsync", true)?;

        let transaction = connection.transaction()?;
        transaction.execute_batch(CREATE)?;
        transaction.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)",
            (ID_KEY, journal.as_bytes().as_slice()),
        )?;
        transaction.pragma_update(None, "user_version", VERSION)?;
        transaction.commit()?;

        connection.close().map_err(|(_, error)| error)?;
        Ok(())
    }

    pub(super) fn check(connection: &Connection, expected: JournalId) -> Result<(), JournalError> {
        Self::check_version(connection)?;

        let found = Self::journal_id(connection)?;

        if found != expected {
            return Err(JournalError::Foreign { expected, found });
        }

        Ok(())
    }

    fn check_version(connection: &Connection) -> Result<(), JournalError> {
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;

        match version {
            VERSION => Ok(()),
            0 => Err(JournalError::NotAJournal("no journal schema")),
            found if found > VERSION => Err(JournalError::SchemaTooNew {
                found,
                supported: VERSION,
            }),
            _ => Err(JournalError::NotAJournal("unknown journal schema")),
        }
    }

    fn journal_id(connection: &Connection) -> Result<JournalId, JournalError> {
        let bytes: Vec<u8> =
            connection.query_row("SELECT value FROM meta WHERE key = ?1", [ID_KEY], |row| {
                row.get(0)
            })?;

        let bytes = bytes
            .try_into()
            .map_err(|_| JournalError::NotAJournal("journal id has the wrong length"))?;

        Ok(JournalId::from_bytes(bytes))
    }
}
