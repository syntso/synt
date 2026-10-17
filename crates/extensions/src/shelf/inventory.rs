use rusqlite::OptionalExtension;

use super::{Installed, Sqlite};
use crate::ExtensionError;

const COLUMNS: &str = "id, version, sha256, manifest_sha256, signed, enabled, installed_at";

#[derive(Debug, Clone)]
pub(crate) struct Inventory {
    sqlite: Sqlite,
}

impl Inventory {
    pub(super) fn new(sqlite: Sqlite) -> Self {
        Self { sqlite }
    }

    pub(crate) fn all(&self) -> Result<Vec<Installed>, ExtensionError> {
        let connection = self.sqlite.lock();
        let mut statement =
            connection.prepare(&format!("SELECT {COLUMNS} FROM extensions ORDER BY id"))?;

        let rows = statement
            .query_map([], Installed::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub(crate) fn find(&self, id: &str) -> Result<Option<Installed>, ExtensionError> {
        let found = self
            .sqlite
            .lock()
            .query_row(
                &format!("SELECT {COLUMNS} FROM extensions WHERE id = ?1"),
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
        let mut connection = self.sqlite.lock();
        let transaction = connection.transaction()?;

        if clear_items {
            transaction.execute(
                "DELETE FROM extension_items WHERE extension = ?1",
                [entry.id().as_str()],
            )?;
        }

        transaction.execute(
            &format!(
                "INSERT OR REPLACE INTO extensions ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
            ),
            (
                entry.id().as_str(),
                entry.version().as_str(),
                entry.sha256(),
                entry.manifest_sha256(),
                entry.signed(),
                entry.enabled(),
                entry.added_at().as_millis(),
            ),
        )?;

        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn set_enabled(&self, id: &str, enabled: bool) -> Result<bool, ExtensionError> {
        let changed = self.sqlite.lock().execute(
            "UPDATE extensions SET enabled = ?2 WHERE id = ?1",
            (id, enabled),
        )?;

        Ok(changed > 0)
    }

    pub(crate) fn forget(&self, id: &str) -> Result<bool, ExtensionError> {
        let mut connection = self.sqlite.lock();
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
    use synt_journal::Timestamp;

    use crate::install::Package;
    use crate::manifest::Manifest;
    use crate::sha256::Sha256;
    use crate::shelf::{Installed, Shelf};

    fn hello() -> Installed {
        let manifest =
            Manifest::parse("id = \"hello\"\nname = \"Hello\"\nversion = \"0.1.0\"\napi = \"0.1\"")
                .unwrap();
        let package = Package::new(manifest, b"wasm".to_vec(), false);

        Installed::new(&package, Sha256::of(b"toml"), Timestamp::from_millis(7))
    }

    #[test]
    fn records_and_lists_extensions() {
        let (_folder, shelf) = Shelf::scratch();
        let inventory = shelf.inventory();
        inventory.record(&hello(), false).unwrap();

        assert_eq!(inventory.all().unwrap(), vec![hello()]);
        assert!(inventory.set_enabled("hello", false).unwrap());
        assert!(!inventory.find("hello").unwrap().unwrap().enabled());
        assert!(!inventory.set_enabled("absent", true).unwrap());
    }

    #[test]
    fn a_replacement_can_clear_storage() {
        let (_folder, shelf) = Shelf::scratch();
        shelf.inventory().record(&hello(), false).unwrap();
        shelf.items().put("hello", "token", b"secret").unwrap();

        shelf.inventory().record(&hello(), false).unwrap();
        assert!(shelf.items().get("hello", "token").unwrap().is_some());

        shelf.inventory().record(&hello(), true).unwrap();
        assert_eq!(shelf.items().get("hello", "token").unwrap(), None);
    }

    #[test]
    fn forgetting_clears_storage() {
        let (_folder, shelf) = Shelf::scratch();
        shelf.inventory().record(&hello(), false).unwrap();
        shelf.items().put("hello", "count", b"1").unwrap();

        assert!(shelf.inventory().forget("hello").unwrap());

        assert_eq!(shelf.items().get("hello", "count").unwrap(), None);
        assert!(shelf.inventory().all().unwrap().is_empty());
    }
}
