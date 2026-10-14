use crate::sha256::Sha256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredRevision {
    pub(crate) revision: Sha256,
    pub(crate) parents: Vec<Sha256>,
    pub(crate) body: Vec<u8>,
}

impl StoredRevision {
    pub(crate) fn new(body: &[u8], parents: Vec<Sha256>) -> Self {
        Self {
            revision: Sha256::of(body),
            parents,
            body: body.to_vec(),
        }
    }
}
