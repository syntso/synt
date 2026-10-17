use tokio_postgres::types::FromSqlOwned;
use tokio_postgres::{GenericClient, Row};

use crate::DbError;
use crate::catalog::{CatalogRow, FromCatalogRow, Schema, TableName};

macro_rules! outside_system {
    () => {
        "n.nspname NOT IN ('pg_catalog', 'information_schema') \
         AND n.nspname NOT LIKE 'pg\\_toast%' AND n.nspname NOT LIKE 'pg\\_temp%'"
    };
}

const TABLES: &str = concat!(
    "SELECT n.nspname::text AS table_schema, c.relname::text AS table_name, \
            CASE c.relkind WHEN 'v' THEN 'view' WHEN 'm' THEN 'materialized view' \
            WHEN 'f' THEN 'foreign table' ELSE 'table' END AS table_kind \
     FROM pg_catalog.pg_class c \
     JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
     WHERE c.relkind IN ('r', 'p', 'v', 'm', 'f') AND ",
    outside_system!(),
    " ORDER BY 1, 2"
);

const COLUMNS: &str = concat!(
    "SELECT n.nspname::text AS table_schema, c.relname::text AS table_name, \
            a.attname::text AS column_name, \
            pg_catalog.format_type(a.atttypid, a.atttypmod) AS data_type, \
            NOT a.attnotnull AS nullable, \
            pg_catalog.pg_get_expr(d.adbin, d.adrelid) AS default_value, \
            NULL::int8 AS key_position \
     FROM pg_catalog.pg_attribute a \
     JOIN pg_catalog.pg_class c ON c.oid = a.attrelid \
     JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
     LEFT JOIN pg_catalog.pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
     WHERE a.attnum > 0 AND NOT a.attisdropped \
       AND c.relkind IN ('r', 'p', 'v', 'm', 'f') AND ",
    outside_system!(),
    " ORDER BY n.nspname, c.relname, a.attnum"
);

const INDEXES: &str = concat!(
    "SELECT n.nspname::text AS table_schema, t.relname::text AS table_name, \
            i.relname::text AS index_name, \
            coalesce(a.attname::text, '(expression)') AS column_name, \
            x.indisunique AS is_unique, x.indisprimary AS is_primary \
     FROM pg_catalog.pg_index x \
     JOIN pg_catalog.pg_class t ON t.oid = x.indrelid \
     JOIN pg_catalog.pg_class i ON i.oid = x.indexrelid \
     JOIN pg_catalog.pg_namespace n ON n.oid = t.relnamespace \
     CROSS JOIN LATERAL unnest(x.indkey::int2[]) WITH ORDINALITY AS k(attnum, position) \
     LEFT JOIN pg_catalog.pg_attribute a ON a.attrelid = t.oid AND a.attnum = k.attnum \
     WHERE ",
    outside_system!(),
    " ORDER BY n.nspname, t.relname, i.relname, k.position"
);

const FOREIGN_KEYS: &str = concat!(
    "SELECT n.nspname::text AS table_schema, t.relname::text AS table_name, \
            c.conname::text AS constraint_key, c.conname::text AS constraint_name, \
            a.attname::text AS column_name, rn.nspname::text AS referenced_schema, \
            r.relname::text AS referenced_table, ra.attname::text AS referenced_column \
     FROM pg_catalog.pg_constraint c \
     JOIN pg_catalog.pg_class t ON t.oid = c.conrelid \
     JOIN pg_catalog.pg_namespace n ON n.oid = t.relnamespace \
     JOIN pg_catalog.pg_class r ON r.oid = c.confrelid \
     JOIN pg_catalog.pg_namespace rn ON rn.oid = r.relnamespace \
     CROSS JOIN LATERAL unnest(c.conkey, c.confkey) WITH ORDINALITY AS k(attnum, refnum, position) \
     JOIN pg_catalog.pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = k.attnum \
     JOIN pg_catalog.pg_attribute ra ON ra.attrelid = c.confrelid AND ra.attnum = k.refnum \
     WHERE c.contype = 'f' AND ",
    outside_system!(),
    " ORDER BY n.nspname, t.relname, c.conname, k.position"
);

pub(super) struct Catalog<'a, C> {
    client: &'a C,
}

impl<'a, C: GenericClient + Sync> Catalog<'a, C> {
    pub(super) fn new(client: &'a C) -> Self {
        Self { client }
    }

    pub(super) async fn tables(&self) -> Result<Vec<TableName>, DbError> {
        self.rows(TABLES).await
    }

    pub(super) async fn schema(&self) -> Result<Schema, DbError> {
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

    async fn rows<T: FromCatalogRow>(&self, sql: &str) -> Result<Vec<T>, DbError> {
        let rows = self.client.query(sql, &[]).await?;

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

fn field<T: FromSqlOwned>(row: &Row, name: &str) -> Result<T, DbError> {
    row.try_get(name).map_err(|_| DbError::UnexpectedRow)
}
