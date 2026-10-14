use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rusqlite::Connection;

use super::schema;
use crate::ExtensionError;

const BUSY_WAIT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub(crate) struct Shelf {
    connection: Arc<Mutex<Connection>>,
}

impl Shelf {
    pub(crate) fn open(path: &Path) -> Result<Self, ExtensionError> {
        let connection = Connection::open(path)?;

        connection.busy_timeout(BUSY_WAIT)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.execute_batch(schema::CREATE)?;
        add_manifest_column(&connection)?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub(super) fn lock(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

pub(super) fn signed_number(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

pub(super) fn unsigned_number(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

fn add_manifest_column(connection: &Connection) -> Result<(), ExtensionError> {
    let present = connection
        .prepare(schema::HAS_MANIFEST_COLUMN)?
        .exists([])?;

    if !present {
        connection.execute(schema::MANIFEST_COLUMN, [])?;
    }

    Ok(())
}
