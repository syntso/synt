use super::hex::{self, hex_id};

hex_id!(RevisionId, "revision id");

impl RevisionId {
    pub(crate) fn from_digest(digest: &[u8]) -> Self {
        Self(hex::encode(digest))
    }
}
