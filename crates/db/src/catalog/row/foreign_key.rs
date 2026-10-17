use crate::DbError;
use crate::catalog::{CatalogRow, ForeignKey, FromCatalogRow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForeignKeyRow {
    pub(crate) schema: Option<String>,
    pub(crate) table: String,
    pub(crate) key: String,
    pub(crate) name: Option<String>,
    pub(crate) column: String,
    pub(crate) referenced_schema: Option<String>,
    pub(crate) referenced_table: String,
    pub(crate) referenced_column: String,
}

impl FromCatalogRow for ForeignKeyRow {
    fn read(row: &impl CatalogRow) -> Result<Self, DbError> {
        Ok(Self {
            schema: row.optional_text("table_schema")?,
            table: row.text("table_name")?,
            key: row.text("constraint_key")?,
            name: row.optional_text("constraint_name")?,
            column: row.text("column_name")?,
            referenced_schema: row.optional_text("referenced_schema")?,
            referenced_table: row.text("referenced_table")?,
            referenced_column: row.text("referenced_column")?,
        })
    }
}

impl From<ForeignKeyRow> for ForeignKey {
    fn from(row: ForeignKeyRow) -> Self {
        Self {
            name: row.name,
            columns: vec![row.column],
            referenced_schema: row.referenced_schema,
            referenced_table: row.referenced_table,
            referenced_columns: vec![row.referenced_column],
            key: row.key,
        }
    }
}
