use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use super::schema;
use crate::chain::JournalId;
use crate::error::JournalError;

const PENDING_SUFFIX: &str = ".pending";
const ROLLBACK_SUFFIX: &str = "-journal";

pub(super) fn create(path: &Path, journal: JournalId) -> Result<Connection, JournalError> {
    let pending = sibling(path, PENDING_SUFFIX);
    remove_stale(&pending)?;

    build(&pending, journal)?;
    fs::rename(&pending, path)?;

    open_writable(path)
}

pub(super) fn open_writable(path: &Path) -> Result<Connection, JournalError> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let connection = Connection::open_with_flags(path, flags)?;

    durable(&connection)?;
    check_schema(&connection)?;

    Ok(connection)
}

pub(super) fn open_read_only(path: &Path) -> Result<Connection, JournalError> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let connection = Connection::open_with_flags(path, flags)?;

    check_schema(&connection)?;
    Ok(connection)
}

pub(super) fn journal_id(connection: &Connection) -> Result<JournalId, JournalError> {
    let bytes: Vec<u8> = connection.query_row(
        "SELECT value FROM meta WHERE key = ?1",
        [schema::ID_KEY],
        |row| row.get(0),
    )?;

    let bytes = bytes
        .try_into()
        .map_err(|_| JournalError::NotAJournal("journal id has the wrong length"))?;
    Ok(JournalId::from_bytes(bytes))
}

fn build(path: &Path, journal: JournalId) -> Result<(), JournalError> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let mut connection = Connection::open_with_flags(path, flags)?;

    connection.pragma_update(None, "auto_vacuum", "INCREMENTAL")?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    connection.pragma_update(None, "fullfsync", true)?;

    let transaction = connection.transaction()?;
    transaction.execute_batch(schema::CREATE)?;
    transaction.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)",
        (schema::ID_KEY, journal.as_bytes().as_slice()),
    )?;
    transaction.pragma_update(None, "user_version", schema::VERSION)?;
    transaction.commit()?;

    connection.close().map_err(|(_, error)| error)?;
    Ok(())
}

fn remove_stale(pending: &Path) -> Result<(), JournalError> {
    for path in [pending.to_owned(), sibling(pending, ROLLBACK_SUFFIX)] {
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

fn durable(connection: &Connection) -> Result<(), JournalError> {
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

fn check_schema(connection: &Connection) -> Result<(), JournalError> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;

    match version {
        schema::VERSION => Ok(()),
        0 => Err(JournalError::NotAJournal("no journal schema")),
        found if found > schema::VERSION => Err(JournalError::SchemaTooNew {
            found,
            supported: schema::VERSION,
        }),
        _ => Err(JournalError::NotAJournal("unknown journal schema")),
    }
}
