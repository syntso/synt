use std::path::PathBuf;

use rusqlite::{OptionalExtension, Row};
use synt_journal::Timestamp;

use super::{Count, Sqlite};
use crate::ExtensionError;
use crate::files::Picked;

const MOST_PICKS: u64 = 256;

#[derive(Debug, Clone)]
pub(crate) struct Picks {
    sqlite: Sqlite,
}

impl Picks {
    pub(super) fn new(sqlite: Sqlite) -> Self {
        Self { sqlite }
    }

    pub(crate) fn all(&self, extension: &str) -> Result<Vec<Picked>, ExtensionError> {
        let connection = self.sqlite.lock();
        let mut statement = connection.prepare(
            "SELECT handle, path, access FROM extension_files WHERE extension = ?1 \
             ORDER BY picked_at, handle",
        )?;

        let picks = statement
            .query_map([extension], Self::picked)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(picks)
    }

    pub(crate) fn find(
        &self,
        extension: &str,
        handle: &str,
    ) -> Result<Option<Picked>, ExtensionError> {
        let found = self
            .sqlite
            .lock()
            .query_row(
                "SELECT handle, path, access FROM extension_files \
                 WHERE extension = ?1 AND handle = ?2",
                [extension, handle],
                Self::picked,
            )
            .optional()?;

        Ok(found)
    }

    pub(crate) fn put(
        &self,
        extension: &str,
        pick: &Picked,
        at: Timestamp,
    ) -> Result<(), ExtensionError> {
        let mut connection = self.sqlite.lock();
        let transaction = connection.transaction()?;

        let count: Count = transaction.query_row(
            "SELECT COUNT(*) FROM extension_files WHERE extension = ?1",
            [extension],
            |row| row.get(0),
        )?;
        if count.get() >= MOST_PICKS {
            return Err(ExtensionError::TooManyPicks {
                extension: extension.to_owned(),
                most: MOST_PICKS,
            });
        }

        transaction.execute(
            "INSERT INTO extension_files (extension, handle, path, access, picked_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                extension,
                pick.handle(),
                pick.path().to_string_lossy(),
                pick.access(),
                at.as_millis(),
            ),
        )?;
        transaction.commit()?;

        Ok(())
    }

    pub(crate) fn forget(&self, extension: &str, handle: &str) -> Result<bool, ExtensionError> {
        let removed = self.sqlite.lock().execute(
            "DELETE FROM extension_files WHERE extension = ?1 AND handle = ?2",
            [extension, handle],
        )?;

        Ok(removed > 0)
    }

    fn picked(row: &Row<'_>) -> rusqlite::Result<Picked> {
        let path: String = row.get("path")?;

        Ok(Picked::stored(
            row.get("handle")?,
            PathBuf::from(path),
            row.get("access")?,
        ))
    }
}
