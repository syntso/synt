use serde::Serialize;

use super::row::{ColumnRow, ForeignKeyRow, IndexRow};
use super::{Table, TableName};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Schema {
    pub tables: Vec<Table>,
}

impl Schema {
    pub(crate) fn of(names: Vec<TableName>) -> Self {
        Self {
            tables: names.into_iter().map(Table::from).collect(),
        }
    }

    #[must_use]
    pub(crate) fn with_columns(mut self, rows: Vec<ColumnRow>) -> Self {
        let mut keys: Vec<&ColumnRow> = rows
            .iter()
            .filter(|row| row.key_position.is_some())
            .collect();
        keys.sort_by_key(|row| row.key_position);

        for key in keys {
            if let Some(table) = self.table_mut(key.schema.as_deref(), &key.table) {
                table.add_primary_key_column(key.column.name.clone());
            }
        }

        for row in rows {
            if let Some(table) = self.table_mut(row.schema.as_deref(), &row.table) {
                table.add_column(row.column);
            }
        }

        self
    }

    #[must_use]
    pub(crate) fn with_indexes(mut self, rows: Vec<IndexRow>) -> Self {
        for row in rows {
            if let Some(table) = self.table_mut(row.schema.as_deref(), &row.table) {
                table.add_index_column(row);
            }
        }

        self
    }

    #[must_use]
    pub(crate) fn with_foreign_keys(mut self, rows: Vec<ForeignKeyRow>) -> Self {
        for row in rows {
            if let Some(table) = self.table_mut(row.schema.as_deref(), &row.table) {
                table.add_foreign_key_column(row);
            }
        }

        self
    }

    fn table_mut(&mut self, schema: Option<&str>, name: &str) -> Option<&mut Table> {
        self.tables.iter_mut().find(|table| table.is(schema, name))
    }
}

#[cfg(test)]
mod tests {
    use super::Schema;
    use crate::catalog::row::{ColumnRow, ForeignKeyRow, IndexRow};
    use crate::catalog::{Column, TableKind, TableName};

    fn lines() -> Schema {
        Schema::of(vec![TableName {
            schema: Some("public".to_owned()),
            name: "lines".to_owned(),
            kind: TableKind::Table,
        }])
    }

    fn index_row(table: &str, index: &str, column: &str, primary: bool) -> IndexRow {
        IndexRow {
            schema: Some("public".to_owned()),
            table: table.to_owned(),
            index: index.to_owned(),
            column: column.to_owned(),
            unique: primary,
            primary,
        }
    }

    fn column_row(name: &str, key_position: Option<i64>) -> ColumnRow {
        ColumnRow {
            schema: Some("public".to_owned()),
            table: "lines".to_owned(),
            column: Column {
                name: name.to_owned(),
                data_type: "int".to_owned(),
                nullable: false,
                default: None,
            },
            key_position,
        }
    }

    #[test]
    fn index_rows_join_into_one_index_and_the_primary_one_names_the_key() {
        let schema = lines().with_indexes(vec![
            index_row("lines", "lines_pkey", "order_id", true),
            index_row("lines", "lines_pkey", "line", true),
            index_row("lines", "by_sku", "sku", false),
        ]);

        let lines = &schema.tables[0];
        assert_eq!(lines.primary_key, ["order_id", "line"]);
        assert_eq!(lines.indexes.len(), 2);
        assert_eq!(lines.indexes[0].columns, ["order_id", "line"]);
    }

    #[test]
    fn foreign_key_rows_with_the_same_key_pair_up_their_columns() {
        let row = |column: &str, referenced: &str| ForeignKeyRow {
            schema: Some("public".to_owned()),
            table: "lines".to_owned(),
            key: "lines_order_fkey".to_owned(),
            name: Some("lines_order_fkey".to_owned()),
            column: column.to_owned(),
            referenced_schema: Some("public".to_owned()),
            referenced_table: "orders".to_owned(),
            referenced_column: referenced.to_owned(),
        };

        let schema =
            lines().with_foreign_keys(vec![row("order_id", "id"), row("region", "region")]);

        let key = &schema.tables[0].foreign_keys[0];
        assert_eq!(key.columns, ["order_id", "region"]);
        assert_eq!(key.referenced_columns, ["id", "region"]);
    }

    #[test]
    fn key_positions_order_the_primary_key_and_columns_keep_their_order() {
        let schema = lines().with_columns(vec![
            column_row("line", Some(2)),
            column_row("note", None),
            column_row("order_id", Some(1)),
        ]);

        let lines = &schema.tables[0];
        assert_eq!(lines.primary_key, ["order_id", "line"]);
        assert_eq!(lines.columns[1].name, "note");
    }

    #[test]
    fn rows_of_unknown_tables_are_ignored() {
        let schema = lines().with_indexes(vec![index_row("ghost", "x", "y", false)]);

        assert!(schema.tables[0].indexes.is_empty());
    }
}
