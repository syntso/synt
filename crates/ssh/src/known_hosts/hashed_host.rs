use data_encoding::BASE64;
use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;

const HASHED_PREFIX: &str = "|1|";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HashedHost {
    salt: Vec<u8>,
    hash: Vec<u8>,
}

impl HashedHost {
    pub(super) fn parse(pattern: &str) -> Option<Self> {
        let encoded = pattern.strip_prefix(HASHED_PREFIX)?;
        let (salt, hash) = encoded.split_once('|')?;

        Some(Self {
            salt: BASE64.decode(salt.as_bytes()).ok()?,
            hash: BASE64.decode(hash.as_bytes()).ok()?,
        })
    }

    pub(super) fn is_hashed(pattern: &str) -> bool {
        pattern.starts_with(HASHED_PREFIX)
    }

    pub(super) fn matches(&self, name: &str) -> bool {
        let Ok(mac) = Hmac::<Sha1>::new_from_slice(&self.salt) else {
            return false;
        };

        mac.chain_update(name.as_bytes())
            .verify_slice(&self.hash)
            .is_ok()
    }
}
