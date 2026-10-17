use serde::Serialize;

use super::TableKind;
use crate::DbError;
use crate::catalog::{CatalogRow, FromCatalogRow};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct TableName {
    pub schema: Option<String>,
    pub name: String,
    pub kind: TableKind,
}

impl FromCatalogRow for TableName {
    fn read(row: &impl CatalogRow) -> Result<Self, DbError> {
        Ok(Self {
            schema: row.optional_text("table_schema")?,
            name: row.text("table_name")?,
            kind: row.text("table_kind")?.parse()?,
        })
    }
}
