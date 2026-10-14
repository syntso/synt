#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForeignKeyPart {
    pub(crate) key: String,
    pub(crate) name: Option<String>,
    pub(crate) column: String,
    pub(crate) referenced_schema: Option<String>,
    pub(crate) referenced_table: String,
    pub(crate) referenced_column: String,
}
