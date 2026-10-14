use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

use super::SecretKey;
use crate::SyncError;
use crate::random::random_bytes;

const NONCE_BYTES: usize = 24;

pub(crate) fn seal(key: &SecretKey, aad: &[u8], plain: &[u8]) -> Result<Vec<u8>, SyncError> {
    let nonce: [u8; NONCE_BYTES] = random_bytes()?;
    let cipher = cipher(key)?;

    let sealed = cipher
        .encrypt(&XNonce::from(nonce), Payload { msg: plain, aad })
        .map_err(|_| SyncError::Crypto("cannot seal"))?;

    Ok([nonce.as_slice(), &sealed].concat())
}

pub(crate) fn open(
    key: &SecretKey,
    aad: &[u8],
    sealed: &[u8],
) -> Result<Zeroizing<Vec<u8>>, SyncError> {
    let Some((nonce, sealed)) = sealed.split_first_chunk::<NONCE_BYTES>() else {
        return Err(SyncError::Crypto("the sealed bytes are too short"));
    };
    let cipher = cipher(key)?;

    cipher
        .decrypt(&XNonce::from(*nonce), Payload { msg: sealed, aad })
        .map(Zeroizing::new)
        .map_err(|_| SyncError::Crypto("the sealed bytes do not open with this key"))
}

fn cipher(key: &SecretKey) -> Result<XChaCha20Poly1305, SyncError> {
    XChaCha20Poly1305::new_from_slice(key.get()).map_err(|_| SyncError::Crypto("bad key length"))
}

#[cfg(test)]
mod tests {
    use super::{open, seal};
    use crate::seal::SecretKey;

    #[test]
    fn sealed_bytes_open_with_the_same_key_and_context() {
        let key = SecretKey::generate().unwrap();

        let sealed = seal(&key, b"context", b"plain text").unwrap();

        assert_eq!(
            open(&key, b"context", &sealed).unwrap().as_slice(),
            b"plain text"
        );
    }

    #[test]
    fn sealing_twice_gives_different_bytes() {
        let key = SecretKey::generate().unwrap();

        let first = seal(&key, b"", b"same").unwrap();
        let second = seal(&key, b"", b"same").unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn another_key_another_context_or_a_flipped_bit_does_not_open() {
        let key = SecretKey::generate().unwrap();
        let sealed = seal(&key, b"context", b"plain text").unwrap();

        let mut flipped = sealed.clone();
        let last = flipped.len() - 1;
        flipped[last] ^= 1;

        assert!(open(&SecretKey::generate().unwrap(), b"context", &sealed).is_err());
        assert!(open(&key, b"other", &sealed).is_err());
        assert!(open(&key, b"context", &flipped).is_err());
        assert!(open(&key, b"context", &sealed[..10]).is_err());
    }
}
