use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transferred {
    pub bytes: u64,
    pub sha256: String,
}

impl Transferred {
    pub(crate) fn hashed(bytes: u64, hasher: Sha256) -> Self {
        Self {
            bytes,
            sha256: HEXLOWER.encode(&hasher.finalize()),
        }
    }
}
