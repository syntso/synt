use rusqlite::{OptionalExtension, Row};

use super::State;
use crate::CoreError;
use crate::secrets::SecretInfo;

impl State {
    pub(crate) fn secrets(&self) -> Result<Vec<SecretInfo>, CoreError> {
        let connection = self.lock();
        let mut statement =
            connection.prepare("SELECT name, bytes, created_at FROM secrets ORDER BY name")?;

        let secrets = statement
            .query_map([], stored_secret)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(secrets)
    }

    pub(crate) fn secret(&self, name: &str) -> Result<Option<SecretInfo>, CoreError> {
        let secret = self
            .lock()
            .query_row(
                "SELECT name, bytes, created_at FROM secrets WHERE name = ?1",
                [name],
                stored_secret,
            )
            .optional()?;

        Ok(secret)
    }

    pub(crate) fn put_secret(&self, secret: &SecretInfo) -> Result<(), CoreError> {
        self.lock().execute(
            "INSERT OR REPLACE INTO secrets (name, bytes, created_at) VALUES (?1, ?2, ?3)",
            (
                &secret.name,
                i64::try_from(secret.bytes).unwrap_or(i64::MAX),
                i64::try_from(secret.created_at).unwrap_or(i64::MAX),
            ),
        )?;

        Ok(())
    }

    pub(crate) fn delete_secret(&self, name: &str) -> Result<(), CoreError> {
        self.lock()
            .execute("DELETE FROM secrets WHERE name = ?1", [name])?;
        self.forget_secret_use(name)
    }

    pub(crate) fn secret_use(&self, name: &str) -> Result<Option<String>, CoreError> {
        let used_for = self
            .lock()
            .query_row(
                "SELECT used_for FROM secret_uses WHERE name = ?1",
                [name],
                |row| row.get(0),
            )
            .optional()?;

        Ok(used_for)
    }

    pub(crate) fn bind_secret_use(&self, name: &str, used_for: &str) -> Result<(), CoreError> {
        self.lock().execute(
            "INSERT OR REPLACE INTO secret_uses (name, used_for) VALUES (?1, ?2)",
            (name, used_for),
        )?;

        Ok(())
    }

    pub(crate) fn forget_secret_use(&self, name: &str) -> Result<(), CoreError> {
        self.lock()
            .execute("DELETE FROM secret_uses WHERE name = ?1", [name])?;

        Ok(())
    }
}

fn stored_secret(row: &Row<'_>) -> rusqlite::Result<SecretInfo> {
    let name: String = row.get("name")?;
    let bytes: i64 = row.get("bytes")?;
    let created_at: i64 = row.get("created_at")?;

    Ok(SecretInfo {
        reference: format!("secret://{name}"),
        name,
        bytes: u64::try_from(bytes).unwrap_or_default(),
        created_at: u64::try_from(created_at).unwrap_or_default(),
    })
}
