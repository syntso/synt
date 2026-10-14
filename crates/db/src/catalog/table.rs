use serde::Serialize;

use super::{Column, ForeignKey, ForeignKeyPart, Index, IndexPart, TableName};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Table {
    pub schema: Option<String>,
    pub name: String,
    pub kind: String,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,
    pub indexes: Vec<Index>,
    pub foreign_keys: Vec<ForeignKey>,
}

impl Table {
    pub(crate) fn named(name: TableName) -> Self {
        Self {
            schema: name.schema,
            name: name.name,
            kind: name.kind,
            columns: Vec::new(),
            primary_key: Vec::new(),
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
        }
    }

    pub(crate) fn is(&self, schema: Option<&str>, name: &str) -> bool {
        self.schema.as_deref() == schema && self.name == name
    }

    pub(crate) fn add_primary_key_column(&mut self, column: String) {
        if !self.primary_key.contains(&column) {
            self.primary_key.push(column);
        }
    }

    pub(crate) fn add_index_part(&mut self, part: IndexPart) {
        if part.primary {
            self.add_primary_key_column(part.column.clone());
        }

        if let Some(index) = self
            .indexes
            .iter_mut()
            .find(|index| index.name == part.index)
        {
            index.columns.push(part.column);
            return;
        }

        self.indexes.push(Index {
            name: part.index,
            columns: vec![part.column],
            unique: part.unique,
            primary: part.primary,
        });
    }

    pub(crate) fn add_foreign_key_part(&mut self, part: ForeignKeyPart) {
        let found = self.foreign_keys.iter_mut().find(|key| key.key == part.key);

        if let Some(key) = found {
            key.columns.push(part.column);
            key.referenced_columns.push(part.referenced_column);
            return;
        }

        self.foreign_keys.push(ForeignKey {
            name: part.name,
            columns: vec![part.column],
            referenced_schema: part.referenced_schema,
            referenced_table: part.referenced_table,
            referenced_columns: vec![part.referenced_column],
            key: part.key,
        });
    }
}
