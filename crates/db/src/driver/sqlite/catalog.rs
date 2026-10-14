use rusqlite::{Connection, Row};

use crate::DbError;
use crate::catalog::{Column, ForeignKeyPart, IndexPart, Schema, TableName};

macro_rules! user_tables {
    () => {
        "m.name NOT LIKE 'sqlite\\_%' ESCAPE '\\'"
    };
}

const TABLES: &str = concat!(
    "SELECT m.name, m.type FROM sqlite_schema m \
     WHERE m.type IN ('table', 'view') AND ",
    user_tables!(),
    " ORDER BY m.name"
);

const COLUMNS: &str = concat!(
    "SELECT m.name, p.name, p.type, p.\"notnull\", p.dflt_value, p.pk \
     FROM sqlite_schema m JOIN pragma_table_info(m.name) p \
     WHERE m.type IN ('table', 'view') AND ",
    user_tables!(),
    " ORDER BY m.name, p.cid"
);

const INDEXES: &str = concat!(
    "SELECT m.name, l.name, coalesce(i.name, '(expression)'), l.\"unique\", l.origin = 'pk' \
     FROM sqlite_schema m JOIN pragma_index_list(m.name) l JOIN pragma_index_info(l.name) i \
     WHERE m.type = 'table' AND ",
    user_tables!(),
    " ORDER BY m.name, l.name, i.seqno"
);

const FOREIGN_KEYS: &str = concat!(
    "SELECT m.name, f.id, f.\"from\", f.\"table\", coalesce(f.\"to\", '') \
     FROM sqlite_schema m JOIN pragma_foreign_key_list(m.name) f \
     WHERE m.type = 'table' AND ",
    user_tables!(),
    " ORDER BY m.name, f.id, f.seq"
);

type ColumnRow = (String, String, String, i64, Option<String>, i64);
type IndexRow = (String, String, String, bool, bool);
type ForeignKeyRow = (String, i64, String, String, String);

pub(super) struct Catalog<'a> {
    connection: &'a Connection,
}

impl<'a> Catalog<'a> {
    pub(super) fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub(super) fn tables(&self) -> Result<Vec<TableName>, DbError> {
        let rows: Vec<(String, String)> = self.rows(TABLES)?;

        let names = rows
            .into_iter()
            .map(|(name, kind)| TableName {
                schema: None,
                name,
                kind,
            })
            .collect();

        Ok(names)
    }

    pub(super) fn schema(&self) -> Result<Schema, DbError> {
        let mut schema = Schema::of(self.tables()?);

        self.add_columns(&mut schema)?;
        self.add_indexes(&mut schema)?;
        self.add_foreign_keys(&mut schema)?;

        Ok(schema)
    }

    fn add_columns(&self, schema: &mut Schema) -> Result<(), DbError> {
        let rows: Vec<ColumnRow> = self.rows(COLUMNS)?;
        let mut keys = Vec::new();

        for (table, name, data_type, not_null, default, key_position) in rows {
            if key_position > 0 {
                keys.push((table.clone(), key_position, name.clone()));
            }

            let column = Column {
                name,
                data_type,
                nullable: not_null == 0,
                default,
            };
            schema.add_column(None, &table, column);
        }

        keys.sort();

        for (table, _, column) in keys {
            schema.add_primary_key_column(None, &table, column);
        }

        Ok(())
    }

    fn add_indexes(&self, schema: &mut Schema) -> Result<(), DbError> {
        let rows: Vec<IndexRow> = self.rows(INDEXES)?;

        for (table, index, column, unique, primary) in rows {
            let part = IndexPart {
                index,
                column,
                unique,
                primary,
            };
            schema.add_index_part(None, &table, part);
        }

        Ok(())
    }

    fn add_foreign_keys(&self, schema: &mut Schema) -> Result<(), DbError> {
        let rows: Vec<ForeignKeyRow> = self.rows(FOREIGN_KEYS)?;

        for (table, key, column, referenced_table, referenced_column) in rows {
            let part = ForeignKeyPart {
                key: key.to_string(),
                name: None,
                column,
                referenced_schema: None,
                referenced_table,
                referenced_column,
            };
            schema.add_foreign_key_part(None, &table, part);
        }

        Ok(())
    }

    fn rows<T>(&self, sql: &str) -> Result<Vec<T>, DbError>
    where
        T: for<'r> TryFrom<&'r Row<'r>, Error = rusqlite::Error>,
    {
        let mut statement = self.connection.prepare(sql)?;

        let rows = statement
            .query_map([], |row| T::try_from(row))?
            .collect::<Result<Vec<T>, _>>()?;

        Ok(rows)
    }
}
