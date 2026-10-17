use rusqlite::types::FromSql;
use rusqlite::{Connection, Row};

use crate::DbError;
use crate::catalog::{CatalogRow, FromCatalogRow, Schema, TableName};

macro_rules! user_tables {
    () => {
        "m.name NOT LIKE 'sqlite\\_%' ESCAPE '\\'"
    };
}

const TABLES: &str = concat!(
    "SELECT NULL AS table_schema, m.name AS table_name, m.type AS table_kind \
     FROM sqlite_schema m \
     WHERE m.type IN ('table', 'view') AND ",
    user_tables!(),
    " ORDER BY m.name"
);

const COLUMNS: &str = concat!(
    "SELECT NULL AS table_schema, m.name AS table_name, p.name AS column_name, \
            p.type AS data_type, NOT p.\"notnull\" AS nullable, \
            p.dflt_value AS default_value, nullif(p.pk, 0) AS key_position \
     FROM sqlite_schema m JOIN pragma_table_info(m.name) p \
     WHERE m.type IN ('table', 'view') AND ",
    user_tables!(),
    " ORDER BY m.name, p.cid"
);

const INDEXES: &str = concat!(
    "SELECT NULL AS table_schema, m.name AS table_name, l.name AS index_name, \
            coalesce(i.name, '(expression)') AS column_name, l.\"unique\" AS is_unique, \
            l.origin = 'pk' AS is_primary \
     FROM sqlite_schema m JOIN pragma_index_list(m.name) l JOIN pragma_index_info(l.name) i \
     WHERE m.type = 'table' AND ",
    user_tables!(),
    " ORDER BY m.name, l.name, i.seqno"
);

const FOREIGN_KEYS: &str = concat!(
    "SELECT NULL AS table_schema, m.name AS table_name, \
            CAST(f.id AS TEXT) AS constraint_key, NULL AS constraint_name, \
            f.\"from\" AS column_name, NULL AS referenced_schema, \
            f.\"table\" AS referenced_table, coalesce(f.\"to\", '') AS referenced_column \
     FROM sqlite_schema m JOIN pragma_foreign_key_list(m.name) f \
     WHERE m.type = 'table' AND ",
    user_tables!(),
    " ORDER BY m.name, f.id, f.seq"
);

pub(super) struct Catalog<'a> {
    connection: &'a Connection,
}

impl<'a> Catalog<'a> {
    pub(super) fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub(super) fn tables(&self) -> Result<Vec<TableName>, DbError> {
        self.rows(TABLES)
    }

    pub(super) fn schema(&self) -> Result<Schema, DbError> {
        let tables = self.tables()?;
        let columns = self.rows(COLUMNS)?;
        let indexes = self.rows(INDEXES)?;
        let foreign_keys = self.rows(FOREIGN_KEYS)?;

        let schema = Schema::of(tables)
            .with_columns(columns)
            .with_indexes(indexes)
            .with_foreign_keys(foreign_keys);

        Ok(schema)
    }

    fn rows<T: FromCatalogRow>(&self, sql: &str) -> Result<Vec<T>, DbError> {
        let mut statement = self.connection.prepare(sql)?;
        let mut rows = statement.query([])?;
        let mut read = Vec::new();

        while let Some(row) = rows.next()? {
            read.push(T::read(row)?);
        }

        Ok(read)
    }
}

impl CatalogRow for Row<'_> {
    fn text(&self, name: &str) -> Result<String, DbError> {
        field(self, name)
    }

    fn optional_text(&self, name: &str) -> Result<Option<String>, DbError> {
        field(self, name)
    }

    fn flag(&self, name: &str) -> Result<bool, DbError> {
        field(self, name)
    }

    fn optional_number(&self, name: &str) -> Result<Option<i64>, DbError> {
        field(self, name)
    }
}

fn field<T: FromSql>(row: &Row<'_>, name: &str) -> Result<T, DbError> {
    row.get(name).map_err(|_| DbError::unexpected_catalog_row())
}
