use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rusqlite::Connection;

use super::schema;
use crate::ExtensionError;

const BUSY_WAIT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub(super) struct Sqlite {
    connection: Arc<Mutex<Connection>>,
}

impl Sqlite {
    pub(super) fn open(path: &Path) -> Result<Self, ExtensionError> {
        let connection = Connection::open(path)?;

        connection.busy_timeout(BUSY_WAIT)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.execute_batch(schema::CREATE)?;
        Self::add_manifest_column(&connection)?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub(super) fn lock(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
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
}
