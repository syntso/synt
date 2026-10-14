use crate::digest::sha256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredRevision {
    pub(crate) revision: String,
    pub(crate) parents: Vec<String>,
    pub(crate) body: Vec<u8>,
}

impl StoredRevision {
    pub(crate) fn new(body: &[u8], parents: Vec<String>) -> Self {
        Self {
            revision: sha256(body),
            parents,
            body: body.to_vec(),
        }
    }
}
