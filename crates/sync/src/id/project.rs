use super::hex::{self, hex_id};
use crate::SyncError;
use crate::random::random_bytes;

hex_id!(ProjectId, "project id");

impl ProjectId {
    pub fn generate() -> Result<Self, SyncError> {
        let bytes = random_bytes::<{ hex::ID_BYTES }>()?;

        Ok(Self(hex::encode(&bytes)))
    }
}
