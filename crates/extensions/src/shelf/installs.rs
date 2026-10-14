use rusqlite::OptionalExtension;

use super::database::signed_number;
use super::{Installed, Shelf};
use crate::ExtensionError;

const INSTALLED_COLUMNS: &str =
    "id, version, sha256, manifest_sha256, signed, enabled, installed_at";

impl Shelf {
    pub(crate) fn installed(&self) -> Result<Vec<Installed>, ExtensionError> {
        let connection = self.lock();
        let mut statement = connection.prepare(&format!(
            "SELECT {INSTALLED_COLUMNS} FROM extensions ORDER BY id"
        ))?;

        let rows = statement
            .query_map([], Installed::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub(crate) fn find(&self, id: &str) -> Result<Option<Installed>, ExtensionError> {
        let found = self
            .lock()
            .query_row(
                &format!("SELECT {INSTALLED_COLUMNS} FROM extensions WHERE id = ?1"),
                [id],
                Installed::from_row,
            )
            .optional()?;

        Ok(found)
    }

    pub(crate) fn record(
        &self,
        entry: &Installed,
        clear_items: bool,
    ) -> Result<(), ExtensionError> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;

        if clear_items {
            transaction.execute(
                "DELETE FROM extension_items WHERE extension = ?1",
                [&entry.id],
            )?;
        }

        transaction.execute(
            &format!(
                "INSERT OR REPLACE INTO extensions ({INSTALLED_COLUMNS}) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
            ),
            (
                &entry.id,
                &entry.version,
                &entry.sha256,
                &entry.manifest_sha256,
                entry.signed,
                entry.enabled,
                signed_number(entry.added_at),
            ),
        )?;

        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn set_enabled(&self, id: &str, enabled: bool) -> Result<bool, ExtensionError> {
        let changed = self.lock().execute(
            "UPDATE extensions SET enabled = ?2 WHERE id = ?1",
            (id, enabled),
        )?;

        Ok(changed > 0)
    }

    pub(crate) fn forget(&self, id: &str) -> Result<bool, ExtensionError> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;

        transaction.execute("DELETE FROM extension_items WHERE extension = ?1", [id])?;
        transaction.execute("DELETE FROM extension_settings WHERE extension = ?1", [id])?;
        transaction.execute("DELETE FROM extension_files WHERE extension = ?1", [id])?;
        let removed = transaction.execute("DELETE FROM extensions WHERE id = ?1", [id])?;

        transaction.commit()?;
        Ok(removed > 0)
    }
}

#[cfg(test)]
mod tests {
    use crate::shelf::{Installed, Shelf};

    fn shelf() -> (tempfile::TempDir, Shelf) {
        let folder = tempfile::tempdir().unwrap();
        let shelf = Shelf::open(&folder.path().join("state.sqlite")).unwrap();

        (folder, shelf)
    }

    fn hello() -> Installed {
        Installed {
            id: "hello".to_owned(),
            version: "0.1.0".to_owned(),
            sha256: "00".to_owned(),
            manifest_sha256: "11".to_owned(),
            signed: false,
            enabled: true,
            added_at: 7,
        }
    }

    #[test]
    fn records_and_lists_extensions() {
        let (_folder, shelf) = shelf();
        shelf.record(&hello(), false).unwrap();

        assert_eq!(shelf.installed().unwrap(), vec![hello()]);
        assert!(shelf.set_enabled("hello", false).unwrap());
        assert!(!shelf.find("hello").unwrap().unwrap().enabled);
        assert!(!shelf.set_enabled("absent", true).unwrap());
    }

    #[test]
    fn a_replacement_can_clear_storage() {
        let (_folder, shelf) = shelf();
        shelf.record(&hello(), false).unwrap();
        shelf.put_item("hello", "token", b"secret").unwrap();

        shelf.record(&hello(), false).unwrap();
        assert!(shelf.item("hello", "token").unwrap().is_some());

        shelf.record(&hello(), true).unwrap();
        assert_eq!(shelf.item("hello", "token").unwrap(), None);
    }

    #[test]
    fn forgetting_clears_storage() {
        let (_folder, shelf) = shelf();
        shelf.record(&hello(), false).unwrap();
        shelf.put_item("hello", "count", b"1").unwrap();

        assert!(shelf.forget("hello").unwrap());

        assert_eq!(shelf.item("hello", "count").unwrap(), None);
        assert!(shelf.installed().unwrap().is_empty());
    }
}
