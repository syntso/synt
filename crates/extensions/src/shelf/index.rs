use rusqlite::OptionalExtension;

use super::Shelf;
use super::database::{signed_number, unsigned_number};
use crate::ExtensionError;

impl Shelf {
    pub(crate) fn accept_index(&self, serial: u64) -> Result<(), ExtensionError> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;

        let seen: Option<i64> = transaction
            .query_row(
                "SELECT serial FROM extension_index WHERE id = 1",
                [],
                |row| row.get("serial"),
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
}

#[cfg(test)]
mod tests {
    use crate::shelf::Shelf;

    #[test]
    fn an_older_index_is_refused() {
        let folder = tempfile::tempdir().unwrap();
        let shelf = Shelf::open(&folder.path().join("state.sqlite")).unwrap();

        shelf.accept_index(5).unwrap();
        shelf.accept_index(5).unwrap();
        shelf.accept_index(9).unwrap();

        assert!(shelf.accept_index(8).is_err());
    }
}
