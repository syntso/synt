use rusqlite::OptionalExtension;

use super::Shelf;
use super::database::unsigned_number;
use crate::ExtensionError;

const STORAGE_LIMIT: u64 = 1024 * 1024;

impl Shelf {
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
                |row| row.get("value"),
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
}

#[cfg(test)]
mod tests {
    use super::STORAGE_LIMIT;
    use crate::shelf::Shelf;

    fn shelf() -> (tempfile::TempDir, Shelf) {
        let folder = tempfile::tempdir().unwrap();
        let shelf = Shelf::open(&folder.path().join("state.sqlite")).unwrap();

        (folder, shelf)
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
}
