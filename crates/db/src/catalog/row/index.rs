use crate::DbError;
use crate::catalog::{CatalogRow, FromCatalogRow, Index};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexRow {
    pub(crate) schema: Option<String>,
    pub(crate) table: String,
    pub(crate) index: String,
    pub(crate) column: String,
    pub(crate) unique: bool,
    pub(crate) primary: bool,
}

impl FromCatalogRow for IndexRow {
    fn read(row: &impl CatalogRow) -> Result<Self, DbError> {
        Ok(Self {
            schema: row.optional_text("table_schema")?,
            table: row.text("table_name")?,
            index: row.text("index_name")?,
            column: row.text("column_name")?,
            unique: row.flag("is_unique")?,
            primary: row.flag("is_primary")?,
        })
    }
}

impl From<IndexRow> for Index {
    fn from(row: IndexRow) -> Self {
        Self {
            name: row.index,
            columns: vec![row.column],
            unique: row.unique,
            primary: row.primary,
        }
    }
}
