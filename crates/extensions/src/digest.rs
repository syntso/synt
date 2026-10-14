use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};

pub(crate) fn sha256(bytes: &[u8]) -> String {
    HEXLOWER.encode(&Sha256::digest(bytes))
}
