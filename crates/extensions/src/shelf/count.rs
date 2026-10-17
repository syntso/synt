use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(super) struct Count(u64);

impl Count {
    pub(super) fn new(value: u64) -> Self {
        Self(value)
    }

    pub(super) fn get(self) -> u64 {
        self.0
    }
}

impl ToSql for Count {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let value = i64::try_from(self.0)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;

        Ok(ToSqlOutput::from(value))
    }
}

impl FromSql for Count {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let number = i64::column_result(value)?;

        u64::try_from(number)
            .map(Self)
            .map_err(|_| FromSqlError::OutOfRange(number))
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::Count;

    #[test]
    fn counts_round_trip_and_negatives_are_refused() {
        let connection = Connection::open_in_memory().unwrap();

        let back: Count = connection
            .query_row("SELECT ?1", [Count::new(42)], |row| row.get(0))
            .unwrap();
        let negative = connection.query_row("SELECT -1", [], |row| row.get::<_, Count>(0));

        assert_eq!(back.get(), 42);
        assert!(negative.is_err());
    }
}
