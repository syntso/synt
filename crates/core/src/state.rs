mod schema;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;

use crate::CoreError;
use crate::locked::locked;

#[derive(Debug, Clone)]
pub struct State {
    connection: Arc<Mutex<Connection>>,
}

impl State {
    pub fn open(path: &Path) -> Result<Self, CoreError> {
        let connection = Connection::open(path)?;

        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        schema::migrate(&connection)?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, Connection> {
        locked(&self.connection)
    }
}
