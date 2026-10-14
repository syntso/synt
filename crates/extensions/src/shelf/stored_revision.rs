#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredRevision {
    pub(crate) revision: String,
    pub(crate) parents: Vec<String>,
    pub(crate) body: Vec<u8>,
}
