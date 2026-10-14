use std::fmt;

use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::SyncError;
use crate::random::random_bytes;

const NONCE_BYTES: usize = 24;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SecretKey(Zeroizing<[u8; SecretKey::BYTES]>);

impl SecretKey {
    pub(crate) const BYTES: usize = 32;

    pub(crate) fn generate() -> Result<Self, SyncError> {
        Ok(Self::new(random_bytes()?))
    }

    pub(crate) fn new(bytes: [u8; Self::BYTES]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    pub(crate) fn from_slice(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; Self::BYTES] = bytes.try_into().ok()?;

        Some(Self::new(bytes))
    }

    pub(crate) fn derive(salt: &[u8], input: &[u8], context: &[u8]) -> Result<Self, SyncError> {
        let mut key = Self::new([0; Self::BYTES]);

        Hkdf::<Sha256>::new(Some(salt), input)
            .expand(context, key.0.as_mut())
            .map_err(|_| SyncError::Crypto("cannot derive a key"))?;

        Ok(key)
    }

    pub(crate) fn get(&self) -> &[u8; Self::BYTES] {
        &self.0
    }

    pub(crate) fn seal(&self, aad: &[u8], plain: &[u8]) -> Result<Vec<u8>, SyncError> {
        let nonce: [u8; NONCE_BYTES] = random_bytes()?;

        let sealed = self
            .cipher()?
            .encrypt(&XNonce::from(nonce), Payload { msg: plain, aad })
            .map_err(|_| SyncError::Crypto("cannot seal"))?;

        Ok([nonce.as_slice(), &sealed].concat())
    }

    pub(crate) fn open(&self, aad: &[u8], sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>, SyncError> {
        let Some((nonce, sealed)) = sealed.split_first_chunk::<NONCE_BYTES>() else {
            return Err(SyncError::Crypto("the sealed bytes are too short"));
        };

        self.cipher()?
            .decrypt(&XNonce::from(*nonce), Payload { msg: sealed, aad })
            .map(Zeroizing::new)
            .map_err(|_| SyncError::Crypto("the sealed bytes do not open with this key"))
    }

    fn cipher(&self) -> Result<XChaCha20Poly1305, SyncError> {
        XChaCha20Poly1305::new_from_slice(self.get())
            .map_err(|_| SyncError::Crypto("bad key length"))
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretKey(..)")
    }
}

#[cfg(test)]
mod tests {
    use super::SecretKey;

    #[test]
    fn sealed_bytes_open_with_the_same_key_and_context() {
        let key = SecretKey::generate().unwrap();

        let sealed = key.seal(b"context", b"plain text").unwrap();

        assert_eq!(
            key.open(b"context", &sealed).unwrap().as_slice(),
            b"plain text"
        );
    }

    #[test]
    fn sealing_twice_gives_different_bytes() {
        let key = SecretKey::generate().unwrap();

        let first = key.seal(b"", b"same").unwrap();
        let second = key.seal(b"", b"same").unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn another_key_another_context_or_a_flipped_bit_does_not_open() {
        let key = SecretKey::generate().unwrap();
        let sealed = key.seal(b"context", b"plain text").unwrap();

        let mut flipped = sealed.clone();
        let last = flipped.len() - 1;
        flipped[last] ^= 1;

        let other = SecretKey::generate().unwrap();

        assert!(other.open(b"context", &sealed).is_err());
        assert!(key.open(b"other", &sealed).is_err());
        assert!(key.open(b"context", &flipped).is_err());
        assert!(key.open(b"context", &sealed[..10]).is_err());
    }

    #[test]
    fn a_derived_key_depends_on_every_input() {
        let key = SecretKey::derive(b"salt", b"input", b"context").unwrap();

        assert_eq!(
            key,
            SecretKey::derive(b"salt", b"input", b"context").unwrap()
        );
        assert_ne!(
            key,
            SecretKey::derive(b"other", b"input", b"context").unwrap()
        );
        assert_ne!(
            key,
            SecretKey::derive(b"salt", b"other", b"context").unwrap()
        );
        assert_ne!(key, SecretKey::derive(b"salt", b"input", b"other").unwrap());
    }
}
