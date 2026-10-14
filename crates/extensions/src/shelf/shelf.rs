use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Row};
use synt_journal::{Clock, SystemClock};

use super::Installed;
use super::schema;
use crate::ExtensionError;

const STORAGE_LIMIT: u64 = 1024 * 1024;

const BUSY_WAIT: Duration = Duration::from_secs(5);
const INSTALLED_COLUMNS: &str =
    "id, version, sha256, manifest_sha256, signed, enabled, installed_at";

#[derive(Debug, Clone)]
pub(crate) struct Shelf {
    connection: Arc<Mutex<Connection>>,
}

impl Shelf {
    pub(crate) fn open(path: &Path) -> Result<Self, ExtensionError> {
        let connection = Connection::open(path)?;

        connection.busy_timeout(BUSY_WAIT)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.execute_batch(schema::CREATE)?;
        add_manifest_column(&connection)?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub(crate) fn installed(&self) -> Result<Vec<Installed>, ExtensionError> {
        let connection = self.lock();
        let mut statement = connection.prepare(&format!(
            "SELECT {INSTALLED_COLUMNS} FROM extensions ORDER BY id"
        ))?;

        let rows = statement
            .query_map([], installed)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub(crate) fn find(&self, id: &str) -> Result<Option<Installed>, ExtensionError> {
        let found = self
            .lock()
            .query_row(
                &format!("SELECT {INSTALLED_COLUMNS} FROM extensions WHERE id = ?1"),
                [id],
                installed,
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

    pub(crate) fn accept_index(&self, serial: u64) -> Result<(), ExtensionError> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;

        let seen: Option<i64> = transaction
            .query_row(
                "SELECT serial FROM extension_index WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let seen = seen.map_or(0, unsigned_number);

        if serial < seen {
            return Err(ExtensionError::Signature(format!(
                "the registry index is older than one already seen (serial {serial} < {seen})"
            )));
        }

        transaction.execute(
            "INSERT OR REPLACE INTO extension_index (id, serial) VALUES (1, ?1)",
            [signed_number(serial)],
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
        let removed = transaction.execute("DELETE FROM extensions WHERE id = ?1", [id])?;

        transaction.commit()?;
        Ok(removed > 0)
    }

    pub(crate) fn item(
        &self,
        extension: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>, ExtensionError> {
        let value = self
            .lock()
            .query_row(
                "SELECT value FROM extension_items WHERE extension = ?1 AND key = ?2",
                [extension, key],
                |row| row.get(0),
            )
            .optional()?;

        Ok(value)
    }

    pub(crate) fn put_item(
        &self,
        extension: &str,
        key: &str,
        value: &[u8],
    ) -> Result<(), ExtensionError> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;

        let others: i64 = transaction.query_row(
            "SELECT COALESCE(SUM(LENGTH(CAST(key AS BLOB)) + LENGTH(value)), 0) \
             FROM extension_items WHERE extension = ?1 AND key != ?2",
            [extension, key],
            |row| row.get(0),
        )?;
        let total = unsigned_number(others) + (key.len() + value.len()) as u64;

        if total > STORAGE_LIMIT {
            return Err(ExtensionError::Refused(format!(
                "the storage of {extension} would hold {total} bytes; the limit is {STORAGE_LIMIT}"
            )));
        }

        transaction.execute(
            "INSERT OR REPLACE INTO extension_items (extension, key, value) VALUES (?1, ?2, ?3)",
            (extension, key, value),
        )?;
        transaction.commit()?;

        Ok(())
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

pub(crate) fn now() -> u64 {
    unsigned_number(SystemClock.now().as_millis())
}

fn add_manifest_column(connection: &Connection) -> Result<(), ExtensionError> {
    let present = connection
        .prepare("SELECT 1 FROM pragma_table_info('extensions') WHERE name = 'manifest_sha256'")?
        .exists([])?;

    if !present {
        connection.execute(schema::MANIFEST_COLUMN, [])?;
    }

    Ok(())
}

fn installed(row: &Row<'_>) -> rusqlite::Result<Installed> {
    Ok(Installed {
        id: row.get(0)?,
        version: row.get(1)?,
        sha256: row.get(2)?,
        manifest_sha256: row.get(3)?,
        signed: row.get(4)?,
        enabled: row.get(5)?,
        added_at: unsigned_number(row.get(6)?),
    })
}

fn signed_number(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn unsigned_number(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{STORAGE_LIMIT, Shelf};
    use crate::shelf::Installed;

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
    fn keeps_items_apart_per_extension() {
        let (_folder, shelf) = shelf();

        shelf.put_item("hello", "count", b"1").unwrap();
        shelf.put_item("other", "count", b"2").unwrap();

        assert_eq!(shelf.item("hello", "count").unwrap(), Some(b"1".to_vec()));
        assert_eq!(shelf.item("other", "count").unwrap(), Some(b"2".to_vec()));
        assert_eq!(shelf.item("hello", "missing").unwrap(), None);
    }

    #[test]
    fn caps_storage_at_one_mebibyte() {
        let (_folder, shelf) = shelf();
        let half = vec![0; usize::try_from(STORAGE_LIMIT / 2).unwrap()];

        shelf.put_item("hello", "a", &half).unwrap();
        assert!(shelf.put_item("hello", "b", &half).is_err());

        shelf.put_item("hello", "a", &half).unwrap();
        shelf.put_item("other", "b", &half).unwrap();
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
    fn an_older_index_is_refused() {
        let (_folder, shelf) = shelf();

        shelf.accept_index(5).unwrap();
        shelf.accept_index(5).unwrap();
        shelf.accept_index(9).unwrap();

        assert!(shelf.accept_index(8).is_err());
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
