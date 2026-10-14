use mysql_async::prelude::{FromRow, Queryable};
use mysql_async::{Conn, Row};

use crate::DbError;
use crate::catalog::{Column, ForeignKeyPart, IndexPart, Schema, TableName};

macro_rules! in_scope {
    () => {
        "(TABLE_SCHEMA = DATABASE() OR (DATABASE() IS NULL AND TABLE_SCHEMA \
         NOT IN ('mysql', 'information_schema', 'performance_schema', 'sys')))"
    };
}

const TABLES: &str = concat!(
    "SELECT TABLE_SCHEMA, TABLE_NAME, \
            CASE TABLE_TYPE WHEN 'VIEW' THEN 'view' WHEN 'SYSTEM VIEW' THEN 'view' \
            ELSE 'table' END \
     FROM information_schema.TABLES WHERE ",
    in_scope!(),
    " ORDER BY TABLE_SCHEMA, TABLE_NAME"
);

const COLUMNS: &str = concat!(
    "SELECT TABLE_SCHEMA, TABLE_NAME, COLUMN_NAME, COLUMN_TYPE, \
            IS_NULLABLE = 'YES', COLUMN_DEFAULT \
     FROM information_schema.COLUMNS WHERE ",
    in_scope!(),
    " ORDER BY TABLE_SCHEMA, TABLE_NAME, ORDINAL_POSITION"
);

const INDEXES: &str = concat!(
    "SELECT TABLE_SCHEMA, TABLE_NAME, INDEX_NAME, COALESCE(COLUMN_NAME, '(expression)'), \
            NON_UNIQUE = 0, INDEX_NAME = 'PRIMARY' \
     FROM information_schema.STATISTICS WHERE ",
    in_scope!(),
    " ORDER BY TABLE_SCHEMA, TABLE_NAME, INDEX_NAME, SEQ_IN_INDEX"
);

const FOREIGN_KEYS: &str = concat!(
    "SELECT TABLE_SCHEMA, TABLE_NAME, CONSTRAINT_NAME, COLUMN_NAME, \
            REFERENCED_TABLE_SCHEMA, REFERENCED_TABLE_NAME, REFERENCED_COLUMN_NAME \
     FROM information_schema.KEY_COLUMN_USAGE \
     WHERE REFERENCED_TABLE_NAME IS NOT NULL AND ",
    in_scope!(),
    " ORDER BY TABLE_SCHEMA, TABLE_NAME, CONSTRAINT_NAME, ORDINAL_POSITION"
);

type ColumnRow = (String, String, String, String, bool, Option<String>);
type IndexRow = (String, String, String, String, bool, bool);
type ForeignKeyRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
);

pub(super) struct Catalog<'a> {
    connection: &'a mut Conn,
}

impl<'a> Catalog<'a> {
    pub(super) fn new(connection: &'a mut Conn) -> Self {
        Self { connection }
    }

    pub(super) async fn tables(&mut self) -> Result<Vec<TableName>, DbError> {
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

    pub(super) async fn schema(&mut self) -> Result<Schema, DbError> {
        let mut schema = Schema::of(self.tables().await?);

        self.add_columns(&mut schema).await?;
        self.add_indexes(&mut schema).await?;
        self.add_foreign_keys(&mut schema).await?;

        Ok(schema)
    }

    async fn add_columns(&mut self, schema: &mut Schema) -> Result<(), DbError> {
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

    async fn add_indexes(&mut self, schema: &mut Schema) -> Result<(), DbError> {
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

    async fn add_foreign_keys(&mut self, schema: &mut Schema) -> Result<(), DbError> {
        let rows: Vec<ForeignKeyRow> = self.rows(FOREIGN_KEYS).await?;

        for (owner, table, name, column, referenced_schema, referenced_table, referenced_column) in
            rows
        {
            let part = ForeignKeyPart {
                key: name.clone(),
                name: Some(name),
                column,
                referenced_schema,
                referenced_table,
                referenced_column,
            };
            schema.add_foreign_key_part(Some(&owner), &table, part);
        }

        Ok(())
    }

    async fn rows<T: FromRow>(&mut self, sql: &str) -> Result<Vec<T>, DbError> {
        let rows: Vec<Row> = self.connection.query(sql).await?;

        rows.into_iter()
            .map(|row| {
                mysql_async::from_row_opt(row).map_err(|_| DbError::unexpected_catalog_row())
            })
            .collect()
    }
}
