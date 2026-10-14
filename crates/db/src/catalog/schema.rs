use serde::Serialize;

use super::{Column, ForeignKeyPart, IndexPart, Table, TableName};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Schema {
    pub tables: Vec<Table>,
}

impl Schema {
    pub(crate) fn of(names: Vec<TableName>) -> Self {
        Self {
            tables: names.into_iter().map(Table::named).collect(),
        }
    }

    pub(crate) fn add_column(&mut self, schema: Option<&str>, table: &str, column: Column) {
        if let Some(table) = self.table_mut(schema, table) {
            table.columns.push(column);
        }
    }

    pub(crate) fn add_primary_key_column(
        &mut self,
        schema: Option<&str>,
        table: &str,
        column: String,
    ) {
        if let Some(table) = self.table_mut(schema, table) {
            table.add_primary_key_column(column);
        }
    }

    pub(crate) fn add_index_part(&mut self, schema: Option<&str>, table: &str, part: IndexPart) {
        if let Some(table) = self.table_mut(schema, table) {
            table.add_index_part(part);
        }
    }

    pub(crate) fn add_foreign_key_part(
        &mut self,
        schema: Option<&str>,
        table: &str,
        part: ForeignKeyPart,
    ) {
        if let Some(table) = self.table_mut(schema, table) {
            table.add_foreign_key_part(part);
        }
    }

    fn table_mut(&mut self, schema: Option<&str>, name: &str) -> Option<&mut Table> {
        self.tables.iter_mut().find(|table| table.is(schema, name))
    }
}

#[cfg(test)]
mod tests {
    use super::Schema;
    use crate::catalog::{ForeignKeyPart, IndexPart, TableName};

    fn table(name: &str) -> TableName {
        TableName {
            schema: Some("public".to_owned()),
            name: name.to_owned(),
            kind: "table".to_owned(),
        }
    }

    fn index_part(index: &str, column: &str, primary: bool) -> IndexPart {
        IndexPart {
            index: index.to_owned(),
            column: column.to_owned(),
            unique: primary,
            primary,
        }
    }

    #[test]
    fn index_parts_join_into_one_index_and_the_primary_one_names_the_key() {
        let mut schema = Schema::of(vec![table("lines")]);

        schema.add_index_part(
            Some("public"),
            "lines",
            index_part("lines_pkey", "order_id", true),
        );
        schema.add_index_part(
            Some("public"),
            "lines",
            index_part("lines_pkey", "line", true),
        );
        schema.add_index_part(Some("public"), "lines", index_part("by_sku", "sku", false));

        let lines = &schema.tables[0];
        assert_eq!(lines.primary_key, ["order_id", "line"]);
        assert_eq!(lines.indexes.len(), 2);
        assert_eq!(lines.indexes[0].columns, ["order_id", "line"]);
    }

    #[test]
    fn foreign_key_parts_with_the_same_key_pair_up_their_columns() {
        let mut schema = Schema::of(vec![table("lines")]);
        let part = |column: &str, referenced: &str| ForeignKeyPart {
            key: "lines_order_fkey".to_owned(),
            name: Some("lines_order_fkey".to_owned()),
            column: column.to_owned(),
            referenced_schema: Some("public".to_owned()),
            referenced_table: "orders".to_owned(),
            referenced_column: referenced.to_owned(),
        };

        schema.add_foreign_key_part(Some("public"), "lines", part("order_id", "id"));
        schema.add_foreign_key_part(Some("public"), "lines", part("region", "region"));

        let key = &schema.tables[0].foreign_keys[0];
        assert_eq!(key.columns, ["order_id", "region"]);
        assert_eq!(key.referenced_columns, ["id", "region"]);
    }

    #[test]
    fn parts_of_unknown_tables_are_ignored() {
        let mut schema = Schema::of(vec![table("lines")]);

        schema.add_index_part(Some("public"), "ghost", index_part("x", "y", false));

        assert!(schema.tables[0].indexes.is_empty());
    }
}
