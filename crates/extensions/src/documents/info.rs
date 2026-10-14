use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DocumentInfo {
    pub(crate) project: String,
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) revision: String,
    pub(crate) size: u64,
}
