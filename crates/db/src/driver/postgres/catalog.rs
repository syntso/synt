use tokio_postgres::GenericClient;

use super::row::FromRow;
use crate::DbError;
use crate::catalog::{Column, ForeignKeyPart, IndexPart, Schema, TableName};

macro_rules! outside_system {
    () => {
        "n.nspname NOT IN ('pg_catalog', 'information_schema') \
         AND n.nspname NOT LIKE 'pg\\_toast%' AND n.nspname NOT LIKE 'pg\\_temp%'"
    };
}

const TABLES: &str = concat!(
    "SELECT n.nspname::text, c.relname::text, \
            CASE c.relkind WHEN 'v' THEN 'view' WHEN 'm' THEN 'materialized view' \
            WHEN 'f' THEN 'foreign table' ELSE 'table' END \
     FROM pg_catalog.pg_class c \
     JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
     WHERE c.relkind IN ('r', 'p', 'v', 'm', 'f') AND ",
    outside_system!(),
    " ORDER BY 1, 2"
);

const COLUMNS: &str = concat!(
    "SELECT n.nspname::text, c.relname::text, a.attname::text, \
            pg_catalog.format_type(a.atttypid, a.atttypmod), NOT a.attnotnull, \
            pg_catalog.pg_get_expr(d.adbin, d.adrelid) \
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
    "SELECT n.nspname::text, t.relname::text, i.relname::text, \
            coalesce(a.attname::text, '(expression)'), x.indisunique, x.indisprimary \
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
    "SELECT n.nspname::text, t.relname::text, c.conname::text, a.attname::text, \
            rn.nspname::text, r.relname::text, ra.attname::text \
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

type ColumnRow = (String, String, String, String, bool, Option<String>);
type IndexRow = (String, String, String, String, bool, bool);
type ForeignKeyRow = (String, String, String, String, String, String, String);

pub(super) struct Catalog<'a, C> {
    client: &'a C,
}

impl<'a, C: GenericClient + Sync> Catalog<'a, C> {
    pub(super) fn new(client: &'a C) -> Self {
        Self { client }
    }

    pub(super) async fn tables(&self) -> Result<Vec<TableName>, DbError> {
        let rows: Vec<(String, String, String)> = self.rows(TABLES).await?;

        let names = rows
            .into_iter()
            .map(|(schema, name, kind)| TableName {
                schema: Some(schema),
                name,
                kind,
            })
            .collect();

        Ok(names)
    }

    pub(super) async fn schema(&self) -> Result<Schema, DbError> {
        let mut schema = Schema::of(self.tables().await?);

        self.add_columns(&mut schema).await?;
        self.add_indexes(&mut schema).await?;
        self.add_foreign_keys(&mut schema).await?;

        Ok(schema)
    }

    async fn add_columns(&self, schema: &mut Schema) -> Result<(), DbError> {
        let rows: Vec<ColumnRow> = self.rows(COLUMNS).await?;

        for (owner, table, name, data_type, nullable, default) in rows {
            let column = Column {
                name,
                data_type,
                nullable,
                default,
            };
            schema.add_column(Some(&owner), &table, column);
        }

        Ok(())
    }

    async fn add_indexes(&self, schema: &mut Schema) -> Result<(), DbError> {
        let rows: Vec<IndexRow> = self.rows(INDEXES).await?;

        for (owner, table, index, column, unique, primary) in rows {
            let part = IndexPart {
                index,
                column,
                unique,
                primary,
            };
            schema.add_index_part(Some(&owner), &table, part);
        }

        Ok(())
    }

    async fn add_foreign_keys(&self, schema: &mut Schema) -> Result<(), DbError> {
        let rows: Vec<ForeignKeyRow> = self.rows(FOREIGN_KEYS).await?;

        for (owner, table, name, column, referenced_schema, referenced_table, referenced_column) in
            rows
        {
            let part = ForeignKeyPart {
                key: name.clone(),
                name: Some(name),
                column,
                referenced_schema: Some(referenced_schema),
                referenced_table,
                referenced_column,
            };
            schema.add_foreign_key_part(Some(&owner), &table, part);
        }

        Ok(())
    }

    async fn rows<T: FromRow>(&self, sql: &str) -> Result<Vec<T>, DbError> {
        let rows = self.client.query(sql, &[]).await?;

        rows.iter()
            .map(|row| T::from_row(row).map_err(|_| DbError::unexpected_catalog_row()))
            .collect()
    }
}
