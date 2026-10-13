use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, Row};

use super::schema;
use super::{NewGrant, StoredGrant};
use crate::CoreError;

#[derive(Debug, Clone)]
pub struct State {
    connection: Arc<Mutex<Connection>>,
}

impl State {
    pub fn open(path: &Path) -> Result<Self, CoreError> {
        let connection = Connection::open(path)?;

        Self::prepare(connection)
    }

    pub(crate) fn open_projects(&self) -> Result<Vec<PathBuf>, CoreError> {
        let connection = self.lock();
        let mut statement =
            connection.prepare("SELECT path FROM open_projects ORDER BY opened_at, path")?;

        let paths = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(paths.into_iter().map(PathBuf::from).collect())
    }

    pub(crate) fn remember_project(&self, path: &Path) -> Result<(), CoreError> {
        self.lock().execute(
            "INSERT OR IGNORE INTO open_projects (path, opened_at) VALUES (?1, ?2)",
            (path.to_string_lossy(), now()),
        )?;

        Ok(())
    }

    pub(crate) fn forget_project(&self, path: &Path) -> Result<(), CoreError> {
        self.lock().execute(
            "DELETE FROM open_projects WHERE path = ?1",
            [path.to_string_lossy()],
        )?;

        Ok(())
    }

    pub(crate) fn grants(&self) -> Result<Vec<StoredGrant>, CoreError> {
        let connection = self.lock();
        let mut statement = connection.prepare(
            "SELECT id, actor, action, resource, destination, created_at FROM grants ORDER BY id",
        )?;

        let grants = statement
            .query_map([], stored_grant)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(grants)
    }

    pub(crate) fn insert_grant(&self, grant: NewGrant) -> Result<StoredGrant, CoreError> {
        let created_at = now();
        let connection = self.lock();

        connection.execute(
            "INSERT INTO grants (actor, action, resource, destination, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                &grant.actor,
                &grant.action,
                &grant.resource,
                &grant.destination,
                created_at,
            ),
        )?;
        let id = connection.last_insert_rowid();

        Ok(StoredGrant {
            id: unsigned(id),
            actor: grant.actor,
            action: grant.action,
            resource: grant.resource,
            destination: grant.destination,
            created_at: unsigned(created_at),
        })
    }

    pub(crate) fn clear_grants(&self) -> Result<(), CoreError> {
        self.lock().execute("DELETE FROM grants", [])?;

        Ok(())
    }

    pub(crate) fn delete_grant(&self, id: u64) -> Result<bool, CoreError> {
        let Ok(id) = i64::try_from(id) else {
            return Ok(false);
        };

        let connection = self.lock();
        let found: Option<i64> = connection
            .query_row("SELECT id FROM grants WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .optional()?;

        if found.is_none() {
            return Ok(false);
        }

        connection.execute("DELETE FROM grants WHERE id = ?1", [id])?;
        Ok(true)
    }

    fn prepare(connection: Connection) -> Result<Self, CoreError> {
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;

        connection.execute_batch(schema::CREATE)?;
        if version == 1 {
            connection.execute_batch(schema::FROM_VERSION_1)?;
        }
        connection.pragma_update(None, "user_version", schema::VERSION)?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

fn stored_grant(row: &Row<'_>) -> rusqlite::Result<StoredGrant> {
    Ok(StoredGrant {
        id: unsigned(row.get(0)?),
        actor: row.get(1)?,
        action: row.get(2)?,
        resource: row.get(3)?,
        destination: row.get(4)?,
        created_at: unsigned(row.get(5)?),
    })
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}

fn unsigned(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}
