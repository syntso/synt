use rusqlite::{OptionalExtension, Row};
use synt_journal::Timestamp;

use super::{SecretInfo, SecretName};
use crate::CoreError;
use crate::state::State;

#[derive(Debug, Clone)]
pub(crate) struct SecretTable {
    state: State,
}

impl SecretTable {
    pub(crate) fn new(state: State) -> Self {
        Self { state }
    }

    pub(crate) fn all(&self) -> Result<Vec<SecretInfo>, CoreError> {
        let connection = self.state.lock();
        let mut statement =
            connection.prepare("SELECT name, bytes, created_at FROM secrets ORDER BY name")?;

        let secrets = statement
            .query_map([], Self::row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(secrets)
    }

    pub(crate) fn find(&self, name: &SecretName) -> Result<Option<SecretInfo>, CoreError> {
        let secret = self
            .state
            .lock()
            .query_row(
                "SELECT name, bytes, created_at FROM secrets WHERE name = ?1",
                [name],
                Self::row,
            )
            .optional()?;

        Ok(secret)
    }

    pub(crate) fn put(&self, secret: &SecretInfo) -> Result<(), CoreError> {
        self.state.lock().execute(
            "INSERT OR REPLACE INTO secrets (name, bytes, created_at) VALUES (?1, ?2, ?3)",
            (
                &secret.name,
                i64::try_from(secret.bytes).unwrap_or(i64::MAX),
                secret.created_at.as_millis(),
            ),
        )?;

        Ok(())
    }

    pub(crate) fn delete(&self, name: &SecretName) -> Result<(), CoreError> {
        self.state
            .lock()
            .execute("DELETE FROM secrets WHERE name = ?1", [name])?;

        self.forget_use(name)
    }

    pub(crate) fn use_of(&self, name: &SecretName) -> Result<Option<String>, CoreError> {
        let used_for = self
            .state
            .lock()
            .query_row(
                "SELECT used_for FROM secret_uses WHERE name = ?1",
                [name],
                |row| row.get(0),
            )
            .optional()?;

        Ok(used_for)
    }

    pub(crate) fn bind_use(&self, name: &SecretName, used_for: &str) -> Result<(), CoreError> {
        self.state.lock().execute(
            "INSERT OR REPLACE INTO secret_uses (name, used_for) VALUES (?1, ?2)",
            (name, used_for),
        )?;

        Ok(())
    }

    pub(crate) fn forget_use(&self, name: &SecretName) -> Result<(), CoreError> {
        self.state
            .lock()
            .execute("DELETE FROM secret_uses WHERE name = ?1", [name])?;

        Ok(())
    }

    fn row(row: &Row<'_>) -> rusqlite::Result<SecretInfo> {
        let name: SecretName = row.get("name")?;
        let bytes: i64 = row.get("bytes")?;

        Ok(SecretInfo {
            reference: name.reference(),
            name,
            bytes: u64::try_from(bytes).unwrap_or_default(),
            created_at: Timestamp::from_millis(row.get("created_at")?),
        })
    }
}
