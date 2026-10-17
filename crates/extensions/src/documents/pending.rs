use crate::sha256::Sha256;
use crate::shelf::StoredRevision;

pub(super) struct Pending {
    body: Vec<u8>,
    parents: Vec<Sha256>,
    merged: bool,
}

impl Pending {
    pub(super) fn plain(body: &[u8], parent: Option<Sha256>) -> Self {
        Self {
            body: body.to_vec(),
            parents: parent.into_iter().collect(),
            merged: false,
        }
    }

    pub(super) fn merged(body: Vec<u8>, parents: Vec<Sha256>) -> Self {
        Self {
            body,
            parents,
            merged: true,
        }
    }

    pub(super) fn body(&self) -> &[u8] {
        &self.body
    }

    pub(super) fn was_merged(&self) -> bool {
        self.merged
    }

    pub(super) fn revision(&self) -> StoredRevision {
        StoredRevision::new(&self.body, self.parents.clone())
    }
}
