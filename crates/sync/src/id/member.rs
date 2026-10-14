use super::hex::{self, hex_id};

hex_id!(MemberId, "member id");

impl MemberId {
    pub(crate) fn from_digest(digest: &[u8]) -> Self {
        Self(hex::encode(digest))
    }
}
