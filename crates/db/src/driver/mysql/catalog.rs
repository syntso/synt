use mysql_async::prelude::{FromValue, Queryable};
use mysql_async::{Conn, Row};

use crate::DbError;
use crate::catalog::{CatalogRow, FromCatalogRow, Schema, TableName};

macro_rules! in_scope {
    () => {
        "(TABLE_SCHEMA = DATABASE() OR (DATABASE() IS NULL AND TABLE_SCHEMA \
         NOT IN ('mysql', 'information_schema', 'performance_schema', 'sys')))"
    };
}

const TABLES: &str = concat!(
    "SELECT TABLE_SCHEMA AS table_schema, TABLE_NAME AS table_name, \
            CASE TABLE_TYPE WHEN 'VIEW' THEN 'view' WHEN 'SYSTEM VIEW' THEN 'view' \
            ELSE 'table' END AS table_kind \
     FROM information_schema.TABLES WHERE ",
    in_scope!(),
    " ORDER BY TABLE_SCHEMA, TABLE_NAME"
);

const COLUMNS: &str = concat!(
    "SELECT TABLE_SCHEMA AS table_schema, TABLE_NAME AS table_name, \
            COLUMN_NAME AS column_name, COLUMN_TYPE AS data_type, \
            IS_NULLABLE = 'YES' AS nullable, COLUMN_DEFAULT AS default_value, \
            NULL AS key_position \
     FROM information_schema.COLUMNS WHERE ",
    in_scope!(),
    " ORDER BY TABLE_SCHEMA, TABLE_NAME, ORDINAL_POSITION"
);

const INDEXES: &str = concat!(
    "SELECT TABLE_SCHEMA AS table_schema, TABLE_NAME AS table_name, \
            INDEX_NAME AS index_name, COALESCE(COLUMN_NAME, '(expression)') AS column_name, \
            NON_UNIQUE = 0 AS is_unique, INDEX_NAME = 'PRIMARY' AS is_primary \
     FROM information_schema.STATISTICS WHERE ",
    in_scope!(),
    " ORDER BY TABLE_SCHEMA, TABLE_NAME, INDEX_NAME, SEQ_IN_INDEX"
);

const FOREIGN_KEYS: &str = concat!(
    "SELECT TABLE_SCHEMA AS table_schema, TABLE_NAME AS table_name, \
            CONSTRAINT_NAME AS constraint_key, CONSTRAINT_NAME AS constraint_name, \
            COLUMN_NAME AS column_name, REFERENCED_TABLE_SCHEMA AS referenced_schema, \
            REFERENCED_TABLE_NAME AS referenced_table, \
            REFERENCED_COLUMN_NAME AS referenced_column \
     FROM information_schema.KEY_COLUMN_USAGE \
     WHERE REFERENCED_TABLE_NAME IS NOT NULL AND ",
    in_scope!(),
    " ORDER BY TABLE_SCHEMA, TABLE_NAME, CONSTRAINT_NAME, ORDINAL_POSITION"
);

pub(super) struct Catalog<'a> {
    connection: &'a mut Conn,
}

impl<'a> Catalog<'a> {
    pub(super) fn new(connection: &'a mut Conn) -> Self {
        Self { connection }
    }

    pub(super) async fn tables(&mut self) -> Result<Vec<TableName>, DbError> {
        self.rows(TABLES).await
    }

    pub(super) async fn schema(&mut self) -> Result<Schema, DbError> {
        let tables = self.tables().await?;
        let columns = self.rows(COLUMNS).await?;
        let indexes = self.rows(INDEXES).await?;
        let foreign_keys = self.rows(FOREIGN_KEYS).await?;

        let schema = Schema::of(tables)
            .with_columns(columns)
            .with_indexes(indexes)
            .with_foreign_keys(foreign_keys);

        Ok(schema)
    }

    async fn rows<T: FromCatalogRow>(&mut self, sql: &str) -> Result<Vec<T>, DbError> {
        let rows: Vec<Row> = self.connection.query(sql).await?;

        rows.iter().map(T::read).collect()
    }
}

impl CatalogRow for Row {
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

fn field<T: FromValue>(row: &Row, name: &str) -> Result<T, DbError> {
    row.get_opt(name)
        .and_then(Result::ok)
        .ok_or(DbError::UnexpectedRow)
}
