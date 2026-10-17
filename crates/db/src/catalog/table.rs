mod kind;
mod name;

use serde::Serialize;

pub use self::kind::TableKind;
pub use self::name::TableName;
use super::row::{ForeignKeyRow, IndexRow};
use super::{Column, ForeignKey, Index};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Table {
    pub schema: Option<String>,
    pub name: String,
    pub kind: TableKind,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,
    pub indexes: Vec<Index>,
    pub foreign_keys: Vec<ForeignKey>,
}

impl Table {
    pub(crate) fn is(&self, schema: Option<&str>, name: &str) -> bool {
        self.schema.as_deref() == schema && self.name == name
    }

    pub(crate) fn add_column(&mut self, column: Column) {
        self.columns.push(column);
    }

    pub(crate) fn add_primary_key_column(&mut self, column: String) {
        if !self.primary_key.contains(&column) {
            self.primary_key.push(column);
        }
    }

    pub(crate) fn add_index_column(&mut self, row: IndexRow) {
        if row.primary {
            self.add_primary_key_column(row.column.clone());
        }

        let found = self
            .indexes
            .iter_mut()
            .find(|index| index.name == row.index);

        match found {
            Some(index) => index.columns.push(row.column),
            None => self.indexes.push(row.into()),
        }
    }

    pub(crate) fn add_foreign_key_column(&mut self, row: ForeignKeyRow) {
        let found = self.foreign_keys.iter_mut().find(|key| key.key == row.key);

        match found {
            Some(key) => {
                key.columns.push(row.column);
                key.referenced_columns.push(row.referenced_column);
            }
            None => self.foreign_keys.push(row.into()),
        }
    }
}

impl From<TableName> for Table {
    fn from(name: TableName) -> Self {
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
}
