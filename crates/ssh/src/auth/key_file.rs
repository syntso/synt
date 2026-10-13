use std::io;
use std::path::{Path, PathBuf};

use russh::keys::{self, PrivateKey, PublicKey};
use zeroize::Zeroizing;

const PUBLIC_SUFFIX: &str = ".pub";

pub(super) enum KeyFile {
    Missing,
    Encrypted,
    Unreadable,
    Unsupported,
    Loaded(Box<PrivateKey>),
}

impl KeyFile {
    pub(super) async fn read(path: &Path) -> io::Result<Self> {
        let text = match tokio::fs::read_to_string(path).await {
            Ok(text) => Zeroizing::new(text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::Missing),
            Err(error) => return Err(error),
        };

        let key = match keys::decode_secret_key(&text, None) {
            Ok(key) if key.algorithm().is_rsa() => Self::Unsupported,
            Ok(key) => Self::Loaded(Box::new(key)),
            Err(keys::Error::KeyIsEncrypted) => Self::Encrypted,
            Err(_) => Self::Unreadable,
        };

        Ok(key)
    }

    pub(super) async fn public_key(path: &Path) -> Option<PublicKey> {
        if let Some(key) = read_public(&public_path(path)).await {
            return Some(key);
        }

        let text = Zeroizing::new(tokio::fs::read_to_string(path).await.ok()?);
        let private = PrivateKey::from_openssh(text.as_bytes()).ok()?;

        Some(private.public_key().clone())
    }
}

fn public_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(PUBLIC_SUFFIX);

    PathBuf::from(name)
}

async fn read_public(path: &Path) -> Option<PublicKey> {
    let text = tokio::fs::read_to_string(path).await.ok()?;

    PublicKey::from_openssh(text.trim()).ok()
}
