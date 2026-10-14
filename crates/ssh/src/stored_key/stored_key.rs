use std::fmt::{self, Debug, Formatter};
use std::io;
use std::path::Path;

use russh::keys::ssh_key::LineEnding;
use russh::keys::ssh_key::private::Ed25519Keypair;
use russh::keys::{self, PrivateKey};
use zeroize::Zeroizing;

use super::KeySummary;
use crate::SshError;
use crate::auth::read_key_text;

const SEED_LEN: usize = 32;

pub struct StoredKey {
    private: Zeroizing<String>,
    summary: KeySummary,
}

impl StoredKey {
    pub async fn import(path: &Path) -> Result<Self, SshError> {
        let text = read_key_text(path)
            .await
            .map_err(|error| unreadable_file(path, &error))?;

        let key = match keys::decode_secret_key(&text, None) {
            Ok(key) => key,
            Err(keys::Error::KeyIsEncrypted) => return Err(encrypted(path)),
            Err(_) => return Err(not_a_private_key(path)),
        };

        Self::from_key(path, &key)
    }

    pub fn generate_ed25519(comment: &str) -> Result<Self, SshError> {
        let mut seed = Zeroizing::new([0; SEED_LEN]);
        getrandom::fill(seed.as_mut_slice()).map_err(|error| SshError::KeyStore {
            reason: format!("the system random source failed: {error}"),
        })?;

        let mut key = PrivateKey::from(Ed25519Keypair::from_seed(&seed));
        key.set_comment(comment);

        Self::encode(&key)
    }

    pub fn summary(&self) -> &KeySummary {
        &self.summary
    }

    pub fn private_bytes(&self) -> &[u8] {
        self.private.as_bytes()
    }

    fn from_key(path: &Path, key: &PrivateKey) -> Result<Self, SshError> {
        if key.is_encrypted() {
            return Err(encrypted(path));
        }

        if key.algorithm().is_rsa() {
            return Err(SshError::KeyStore {
                reason: format!(
                    "{} is an RSA key; Synt keeps ed25519 and ecdsa keys, and RSA keys work \
                     through ssh-agent only",
                    path.display()
                ),
            });
        }

        Self::encode(key)
    }

    fn encode(key: &PrivateKey) -> Result<Self, SshError> {
        let private = key
            .to_openssh(LineEnding::LF)
            .map_err(|_| SshError::KeyStore {
                reason: "the key could not be written in OpenSSH form".to_owned(),
            })?;

        Ok(Self {
            private,
            summary: KeySummary::of(key),
        })
    }
}

impl Debug for StoredKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoredKey")
            .field("summary", &self.summary)
            .finish_non_exhaustive()
    }
}

pub(crate) fn decode_private_key(bytes: &[u8]) -> Option<PrivateKey> {
    let text = std::str::from_utf8(bytes).ok()?;
    let key = keys::decode_secret_key(text, None).ok()?;

    (!key.is_encrypted()).then_some(key)
}

fn unreadable_file(path: &Path, error: &io::Error) -> SshError {
    let reason = match error.kind() {
        io::ErrorKind::NotFound => format!("there is no file at {}", path.display()),
        io::ErrorKind::InvalidData => error.to_string(),
        _ => format!("{} cannot be read: {error}", path.display()),
    };

    SshError::KeyStore { reason }
}

fn not_a_private_key(path: &Path) -> SshError {
    SshError::KeyStore {
        reason: format!(
            "{} is not a private key Synt can read; give an OpenSSH or PEM private key file, \
             not a .pub file",
            path.display()
        ),
    }
}

fn encrypted(path: &Path) -> SshError {
    SshError::KeyStore {
        reason: format!(
            "{} is protected by a passphrase; Synt cannot take passphrases yet, so decrypt a \
             copy first (ssh-keygen -p -N \"\" -f <copy>), import the copy and delete it",
            path.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{StoredKey, decode_private_key};
    use crate::SshError;

    #[test]
    fn a_generated_key_decodes_back_to_the_same_public_key() {
        let stored = StoredKey::generate_ed25519("test").expect("generate");
        let decoded = decode_private_key(stored.private_bytes()).expect("decode");

        assert_eq!(stored.summary().kind, "ssh-ed25519");
        assert!(stored.summary().fingerprint.starts_with("SHA256:"));
        assert_eq!(
            decoded.public_key().to_openssh().expect("public"),
            stored.summary().public_key
        );
    }

    #[test]
    fn debug_output_leaves_the_private_key_out() {
        let stored = StoredKey::generate_ed25519("test").expect("generate");
        let shown = format!("{stored:?}");

        assert!(!shown.contains("PRIVATE KEY"));
    }

    #[test]
    fn decoding_refuses_text_that_is_not_a_key() {
        assert!(decode_private_key(b"not a key").is_none());
    }

    #[tokio::test]
    async fn an_imported_key_keeps_its_public_half() {
        let folder = tempfile::tempdir().expect("folder");
        let path = folder.path().join("id_ed25519");
        let generated = StoredKey::generate_ed25519("imported").expect("generate");
        fs::write(&path, generated.private_bytes()).expect("write");

        let imported = StoredKey::import(&path).await.expect("import");

        assert_eq!(imported.summary(), generated.summary());
    }

    #[tokio::test]
    async fn a_public_key_file_is_refused() {
        let folder = tempfile::tempdir().expect("folder");
        let path = folder.path().join("id_ed25519.pub");
        let generated = StoredKey::generate_ed25519("public").expect("generate");
        fs::write(&path, &generated.summary().public_key).expect("write");

        let refused = StoredKey::import(&path).await;

        assert!(
            matches!(refused, Err(SshError::KeyStore { reason }) if reason.contains("not a private key"))
        );
    }

    #[tokio::test]
    async fn a_missing_file_is_refused() {
        let folder = tempfile::tempdir().expect("folder");

        let refused = StoredKey::import(&folder.path().join("none")).await;

        assert!(
            matches!(refused, Err(SshError::KeyStore { reason }) if reason.contains("no file"))
        );
    }
}
