use rusqlite::OptionalExtension;

use super::{Count, Sqlite};
use crate::ExtensionError;

#[derive(Debug, Clone)]
pub(crate) struct IndexSerial {
    sqlite: Sqlite,
}

impl IndexSerial {
    pub(super) fn new(sqlite: Sqlite) -> Self {
        Self { sqlite }
    }

    pub(crate) fn accept(&self, serial: u64) -> Result<(), ExtensionError> {
        let mut connection = self.sqlite.lock();
        let transaction = connection.transaction()?;

        let seen: Option<Count> = transaction
            .query_row(
                "SELECT serial FROM extension_index WHERE id = 1",
                [],
                |row| row.get("serial"),
            )
            .optional()?;
        let seen = seen.unwrap_or_default().get();

        if serial < seen {
            return Err(ExtensionError::IndexRolledBack { serial, seen });
        }

        transaction.execute(
            "INSERT OR REPLACE INTO extension_index (id, serial) VALUES (1, ?1)",
            [Count::new(serial)],
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
        let (_folder, shelf) = Shelf::scratch();
        let index = shelf.index();

        index.accept(5).unwrap();
        index.accept(5).unwrap();
        index.accept(9).unwrap();

        assert!(index.accept(8).is_err());
    }
}
