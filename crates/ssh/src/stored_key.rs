mod summary;
mod vault;

use std::fmt::{self, Debug, Formatter};
use std::path::Path;

use russh::keys::ssh_key::LineEnding;
use russh::keys::ssh_key::private::Ed25519Keypair;
use russh::keys::{self, PrivateKey};
use zeroize::Zeroizing;

pub use summary::KeySummary;
pub use vault::{KeyVault, VaultError};

use crate::SshError;
use crate::authenticator::KeyText;

const SEED_LEN: usize = 32;

pub struct StoredKey {
    private: Zeroizing<String>,
    summary: KeySummary,
}

impl StoredKey {
    pub async fn import(path: &Path) -> Result<Self, SshError> {
        let text = KeyText::read(path).await?;

        let key = match text.decode() {
            Ok(key) => key,
            Err(keys::Error::KeyIsEncrypted) => {
                return Err(SshError::PassphraseKey {
                    path: path.to_owned(),
                });
            }
            Err(_) => {
                return Err(SshError::NotPrivateKey {
                    path: path.to_owned(),
                });
            }
        };

        Self::from_key(path, &key)
    }

    pub fn generate_ed25519(comment: &str) -> Result<Self, SshError> {
        let mut seed = Zeroizing::new([0; SEED_LEN]);
        getrandom::fill(seed.as_mut_slice()).map_err(SshError::Random)?;

        let mut key = PrivateKey::from(Ed25519Keypair::from_seed(&seed));
        key.set_comment(comment);

        Self::encode(&key)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Option<PrivateKey> {
        let text = std::str::from_utf8(bytes).ok()?;
        let key = keys::decode_secret_key(text, None).ok()?;

        (!key.is_encrypted()).then_some(key)
    }

    pub fn summary(&self) -> &KeySummary {
        &self.summary
    }

    pub fn private_bytes(&self) -> &[u8] {
        self.private.as_bytes()
    }

    fn from_key(path: &Path, key: &PrivateKey) -> Result<Self, SshError> {
        if key.is_encrypted() {
            return Err(SshError::PassphraseKey {
                path: path.to_owned(),
            });
        }

        if key.algorithm().is_rsa() {
            return Err(SshError::RsaKey {
                path: path.to_owned(),
            });
        }

        Self::encode(key)
    }

    fn encode(key: &PrivateKey) -> Result<Self, SshError> {
        let private = key
            .to_openssh(LineEnding::LF)
            .map_err(|_| SshError::KeyEncoding)?;

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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::StoredKey;
    use crate::SshError;

    #[test]
    fn a_generated_key_decodes_back_to_the_same_public_key() {
        let stored = StoredKey::generate_ed25519("test").expect("generate");
        let decoded = StoredKey::decode(stored.private_bytes()).expect("decode");

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
        assert!(StoredKey::decode(b"not a key").is_none());
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

        assert!(matches!(refused, Err(SshError::NotPrivateKey { .. })));
        assert!(
            refused
                .expect_err("refused")
                .to_string()
                .contains("not a private key")
        );
    }

    #[tokio::test]
    async fn a_missing_file_is_refused() {
        let folder = tempfile::tempdir().expect("folder");

        let refused = StoredKey::import(&folder.path().join("none")).await;

        assert!(matches!(refused, Err(SshError::NoKeyFile { .. })));
        assert!(refused.expect_err("refused").to_string().contains("no file"));
    }
}
