use rusqlite::OptionalExtension;

use super::{Count, Sqlite};
use crate::ExtensionError;

const STORAGE_LIMIT: u64 = 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct Items {
    sqlite: Sqlite,
}

impl Items {
    pub(super) fn new(sqlite: Sqlite) -> Self {
        Self { sqlite }
    }

    pub(crate) fn get(
        &self,
        extension: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>, ExtensionError> {
        let value = self
            .sqlite
            .lock()
            .query_row(
                "SELECT value FROM extension_items WHERE extension = ?1 AND key = ?2",
                [extension, key],
                |row| row.get("value"),
            )
            .optional()?;

        Ok(value)
    }

    pub(crate) fn put(
        &self,
        extension: &str,
        key: &str,
        value: &[u8],
    ) -> Result<(), ExtensionError> {
        let mut connection = self.sqlite.lock();
        let transaction = connection.transaction()?;

        let others: Count = transaction.query_row(
            "SELECT COALESCE(SUM(LENGTH(CAST(key AS BLOB)) + LENGTH(value)), 0) \
             FROM extension_items WHERE extension = ?1 AND key != ?2",
            [extension, key],
            |row| row.get(0),
        )?;
        let added = u64::try_from(key.len() + value.len()).unwrap_or(u64::MAX);
        let total = others.get().saturating_add(added);

        if total > STORAGE_LIMIT {
            return Err(ExtensionError::StorageFull {
                extension: extension.to_owned(),
                total,
                limit: STORAGE_LIMIT,
            });
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

    #[test]
    fn keeps_items_apart_per_extension() {
        let (_folder, shelf) = Shelf::scratch();
        let items = shelf.items();

        items.put("hello", "count", b"1").unwrap();
        items.put("other", "count", b"2").unwrap();

        assert_eq!(items.get("hello", "count").unwrap(), Some(b"1".to_vec()));
        assert_eq!(items.get("other", "count").unwrap(), Some(b"2".to_vec()));
        assert_eq!(items.get("hello", "missing").unwrap(), None);
    }

    #[test]
    fn caps_storage_at_one_mebibyte() {
        let (_folder, shelf) = Shelf::scratch();
        let items = shelf.items();
        let half = vec![0; usize::try_from(STORAGE_LIMIT / 2).unwrap()];

        items.put("hello", "a", &half).unwrap();
        assert!(items.put("hello", "b", &half).is_err());

        items.put("hello", "a", &half).unwrap();
        items.put("other", "b", &half).unwrap();
    }
}
