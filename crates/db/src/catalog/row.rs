mod column;
mod foreign_key;
mod index;

use crate::DbError;

pub(crate) use column::ColumnRow;
pub(crate) use foreign_key::ForeignKeyRow;
pub(crate) use index::IndexRow;

pub(crate) trait CatalogRow {
    fn text(&self, name: &str) -> Result<String, DbError>;

    fn optional_text(&self, name: &str) -> Result<Option<String>, DbError>;

    fn flag(&self, name: &str) -> Result<bool, DbError>;

    fn optional_number(&self, name: &str) -> Result<Option<i64>, DbError>;
}

pub(crate) trait FromCatalogRow: Sized {
    fn read(row: &impl CatalogRow) -> Result<Self, DbError>;
}
