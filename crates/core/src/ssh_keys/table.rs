use rusqlite::{OptionalExtension, Row};
use synt_journal::Timestamp;

use super::{KeyInfo, KeyName};
use crate::CoreError;
use crate::state::State;

const COLUMNS: &str = "name, kind, public_key, fingerprint, created_at";

#[derive(Debug, Clone)]
pub(crate) struct SshKeyTable {
    state: State,
}

impl SshKeyTable {
    pub(crate) fn new(state: State) -> Self {
        Self { state }
    }

    pub(crate) fn all(&self) -> Result<Vec<KeyInfo>, CoreError> {
        let connection = self.state.lock();
        let mut statement =
            connection.prepare(&format!("SELECT {COLUMNS} FROM ssh_keys ORDER BY name"))?;

        let keys = statement
            .query_map([], Self::row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(keys)
    }

    pub(crate) fn find(&self, name: &KeyName) -> Result<Option<KeyInfo>, CoreError> {
        let key = self
            .state
            .lock()
            .query_row(
                &format!("SELECT {COLUMNS} FROM ssh_keys WHERE name = ?1"),
                [name.as_str()],
                Self::row,
            )
            .optional()?;

        Ok(key)
    }

    pub(crate) fn insert(&self, key: &KeyInfo) -> Result<(), CoreError> {
        self.state.lock().execute(
            &format!("INSERT INTO ssh_keys ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5)"),
            (
                key.name.as_str(),
                &key.kind,
                &key.public_key,
                &key.fingerprint,
                key.created_at.as_millis(),
            ),
        )?;

        Ok(())
    }

    pub(crate) fn delete(&self, name: &KeyName) -> Result<(), CoreError> {
        self.state
            .lock()
            .execute("DELETE FROM ssh_keys WHERE name = ?1", [name.as_str()])?;

        Ok(())
    }

    fn row(row: &Row<'_>) -> rusqlite::Result<KeyInfo> {
        let name: KeyName = State::vault_name(row, "name")?;

        Ok(KeyInfo {
            reference: name.reference(),
            name,
            kind: row.get("kind")?,
            public_key: row.get("public_key")?,
            fingerprint: row.get("fingerprint")?,
            created_at: Timestamp::from_millis(row.get("created_at")?),
        })
    }
}
