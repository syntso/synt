use rusqlite::{OptionalExtension, Row};

use super::Shelf;
use super::database::signed_number;
use crate::ExtensionError;
use crate::clock::now;
use crate::manifest::FileAccess;
use crate::picked::Picked;

const MOST_PICKS: i64 = 256;

impl Shelf {
    pub(crate) fn picks(&self, extension: &str) -> Result<Vec<Picked>, ExtensionError> {
        let connection = self.lock();
        let mut statement = connection.prepare(
            "SELECT handle, path, access FROM extension_files WHERE extension = ?1 \
             ORDER BY picked_at, handle",
        )?;

        let picks = statement
            .query_map([extension], picked)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(picks)
    }

    pub(crate) fn pick(
        &self,
        extension: &str,
        handle: &str,
    ) -> Result<Option<Picked>, ExtensionError> {
        let found = self
            .lock()
            .query_row(
                "SELECT handle, path, access FROM extension_files \
                 WHERE extension = ?1 AND handle = ?2",
                [extension, handle],
                picked,
            )
            .optional()?;

        Ok(found)
    }

    pub(crate) fn put_pick(&self, extension: &str, pick: &Picked) -> Result<(), ExtensionError> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;

        let count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM extension_files WHERE extension = ?1",
            [extension],
            |row| row.get(0),
        )?;
        if count >= MOST_PICKS {
            return Err(ExtensionError::Refused(format!(
                "{extension} already holds {MOST_PICKS} picked files; forget some first"
            )));
        }

        transaction.execute(
            "INSERT INTO extension_files (extension, handle, path, access, picked_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                extension,
                &pick.handle,
                &pick.path,
                pick.access.as_str(),
                signed_number(now()),
            ),
        )?;
        transaction.commit()?;

        Ok(())
    }

    pub(crate) fn forget_pick(
        &self,
        extension: &str,
        handle: &str,
    ) -> Result<bool, ExtensionError> {
        let removed = self.lock().execute(
            "DELETE FROM extension_files WHERE extension = ?1 AND handle = ?2",
            [extension, handle],
        )?;

        Ok(removed > 0)
    }
}

fn picked(row: &Row<'_>) -> rusqlite::Result<Picked> {
    let path: String = row.get("path")?;
    let access: String = row.get("access")?;

    Ok(Picked::stored(
        row.get("handle")?,
        path,
        FileAccess::parse(&access).unwrap_or(FileAccess::Read),
    ))
}
