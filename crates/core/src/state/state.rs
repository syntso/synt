use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension, Row};

use super::schema;
use super::{NewGrant, StoredGrant};
use crate::CoreError;
use crate::clock::unix_millis;
use crate::locked::locked;
use crate::passwords::{PasswordInfo, PasswordName};
use crate::ssh_keys::{KeyInfo, KeyName};

const GRANT_COLUMNS: &str = "id, actor, action, resource, destination, created_at";
const SSH_KEY_COLUMNS: &str = "name, kind, public_key, fingerprint, created_at";

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
            (path.to_string_lossy(), unix_millis()),
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
        let mut statement =
            connection.prepare(&format!("SELECT {GRANT_COLUMNS} FROM grants ORDER BY id"))?;

        let grants = statement
            .query_map([], stored_grant)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(grants)
    }

    pub(crate) fn insert_grant(&self, grant: NewGrant) -> Result<StoredGrant, CoreError> {
        let created_at = unix_millis();
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

    pub(crate) fn ssh_keys(&self) -> Result<Vec<KeyInfo>, CoreError> {
        let connection = self.lock();
        let mut statement = connection.prepare(&format!(
            "SELECT {SSH_KEY_COLUMNS} FROM ssh_keys ORDER BY name"
        ))?;

        let keys = statement
            .query_map([], stored_ssh_key)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(keys)
    }

    pub(crate) fn ssh_key(&self, name: &str) -> Result<Option<KeyInfo>, CoreError> {
        let key = self
            .lock()
            .query_row(
                &format!("SELECT {SSH_KEY_COLUMNS} FROM ssh_keys WHERE name = ?1"),
                [name],
                stored_ssh_key,
            )
            .optional()?;

        Ok(key)
    }

    pub(crate) fn insert_ssh_key(&self, key: &KeyInfo) -> Result<(), CoreError> {
        self.lock().execute(
            &format!("INSERT INTO ssh_keys ({SSH_KEY_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5)"),
            (
                &key.name,
                &key.kind,
                &key.public_key,
                &key.fingerprint,
                i64::try_from(key.created_at).unwrap_or(i64::MAX),
            ),
        )?;

        Ok(())
    }

    pub(crate) fn delete_ssh_key(&self, name: &str) -> Result<(), CoreError> {
        self.lock()
            .execute("DELETE FROM ssh_keys WHERE name = ?1", [name])?;

        Ok(())
    }

    pub(crate) fn passwords(&self) -> Result<Vec<PasswordInfo>, CoreError> {
        let connection = self.lock();
        let mut statement =
            connection.prepare("SELECT name, updated_at FROM passwords ORDER BY name")?;

        let passwords = statement
            .query_map([], stored_password)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(passwords)
    }

    pub(crate) fn password(&self, name: &str) -> Result<Option<PasswordInfo>, CoreError> {
        let password = self
            .lock()
            .query_row(
                "SELECT name, updated_at FROM passwords WHERE name = ?1",
                [name],
                stored_password,
            )
            .optional()?;

        Ok(password)
    }

    pub(crate) fn save_password(&self, password: &PasswordInfo) -> Result<(), CoreError> {
        self.lock().execute(
            "INSERT INTO passwords (name, updated_at) VALUES (?1, ?2) \
             ON CONFLICT (name) DO UPDATE SET updated_at = excluded.updated_at",
            (
                &password.name,
                i64::try_from(password.updated_at).unwrap_or(i64::MAX),
            ),
        )?;

        Ok(())
    }

    pub(crate) fn delete_password(&self, name: &str) -> Result<(), CoreError> {
        self.lock()
            .execute("DELETE FROM passwords WHERE name = ?1", [name])?;

        Ok(())
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
        locked(&self.connection)
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

fn stored_ssh_key(row: &Row<'_>) -> rusqlite::Result<KeyInfo> {
    let name: String = row.get(0)?;

    Ok(KeyInfo {
        reference: KeyName::reference(&name),
        name,
        kind: row.get(1)?,
        public_key: row.get(2)?,
        fingerprint: row.get(3)?,
        created_at: unsigned(row.get(4)?),
    })
}

fn stored_password(row: &Row<'_>) -> rusqlite::Result<PasswordInfo> {
    let name: String = row.get(0)?;
    let reference = PasswordName::parse(&name)
        .map(|parsed| parsed.reference())
        .unwrap_or_default();

    Ok(PasswordInfo {
        name,
        reference,
        updated_at: unsigned(row.get(1)?),
    })
}

fn unsigned(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}
