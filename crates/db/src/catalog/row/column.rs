use crate::DbError;
use crate::catalog::{CatalogRow, Column, FromCatalogRow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColumnRow {
    pub(crate) schema: Option<String>,
    pub(crate) table: String,
    pub(crate) column: Column,
    pub(crate) key_position: Option<i64>,
}

impl FromCatalogRow for ColumnRow {
    fn read(row: &impl CatalogRow) -> Result<Self, DbError> {
        let column = Column {
            name: row.text("column_name")?,
            data_type: row.text("data_type")?,
            nullable: row.flag("nullable")?,
            default: row.optional_text("default_value")?,
        };

        Ok(Self {
            schema: row.optional_text("table_schema")?,
            table: row.text("table_name")?,
            column,
            key_position: row.optional_number("key_position")?,
        })
    }
}
