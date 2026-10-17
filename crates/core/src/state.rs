mod schema;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::types::Type;
use rusqlite::{Connection, Row};
use synt_platform::{Lock, VaultKind, VaultName};

use crate::CoreError;

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
        self.connection.locked()
    }

    pub(crate) fn vault_name<K: VaultKind>(
        row: &Row<'_>,
        column: &str,
    ) -> rusqlite::Result<VaultName<K>> {
        let text: String = row.get(column)?;

        VaultName::parse(&text).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
        })
    }
}
