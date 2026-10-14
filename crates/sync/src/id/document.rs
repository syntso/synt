use super::hex::{self, hex_id};

hex_id!(DocumentId, "document id");

impl DocumentId {
    pub(crate) fn from_digest(digest: &[u8]) -> Self {
        Self(hex::encode(digest))
    }
}
