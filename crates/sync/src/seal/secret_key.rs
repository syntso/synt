use std::fmt;

use zeroize::Zeroizing;

use crate::SyncError;
use crate::random::random_bytes;

pub(crate) const SECRET_KEY_BYTES: usize = 32;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SecretKey(Zeroizing<[u8; SECRET_KEY_BYTES]>);

impl SecretKey {
    pub(crate) fn generate() -> Result<Self, SyncError> {
        Ok(Self::new(random_bytes()?))
    }

    pub(crate) fn new(bytes: [u8; SECRET_KEY_BYTES]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    pub(crate) fn from_slice(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; SECRET_KEY_BYTES] = bytes.try_into().ok()?;

        Some(Self::new(bytes))
    }

    pub(crate) fn get(&self) -> &[u8; SECRET_KEY_BYTES] {
        &self.0
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretKey(..)")
    }
}
