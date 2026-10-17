use std::path::{Path, PathBuf};

use russh::keys::{self, PrivateKey, PublicKey};

use super::KeyText;
use crate::SshError;

const PUBLIC_SUFFIX: &str = ".pub";

pub(in crate::authenticator) enum KeyFile {
    Missing,
    Encrypted,
    Unreadable,
    Unsupported,
    Loaded(Box<PrivateKey>),
}

impl KeyFile {
    pub(in crate::authenticator) async fn read(path: &Path) -> Result<Self, SshError> {
        let text = match KeyText::read(path).await {
            Ok(text) => text,
            Err(SshError::NoKeyFile { .. }) => return Ok(Self::Missing),
            Err(SshError::NotKeyFile { .. }) => return Ok(Self::Unreadable),
            Err(error) => return Err(error),
        };

        let file = match text.decode() {
            Ok(key) if key.algorithm().is_rsa() => Self::Unsupported,
            Ok(key) => Self::Loaded(Box::new(key)),
            Err(keys::Error::KeyIsEncrypted) => Self::Encrypted,
            Err(_) => Self::Unreadable,
        };

        Ok(file)
    }

    pub(in crate::authenticator) async fn exists(path: &Path) -> bool {
        tokio::fs::try_exists(path).await.unwrap_or(false)
    }

    pub(in crate::authenticator) async fn public_key(path: &Path) -> Option<PublicKey> {
        if let Some(key) = Self::read_public(&Self::public_path(path)).await {
            return Some(key);
        }

        let text = KeyText::read(path).await.ok()?;

        text.public_half()
    }

    fn public_path(path: &Path) -> PathBuf {
        let mut name = path.as_os_str().to_owned();
        name.push(PUBLIC_SUFFIX);

        PathBuf::from(name)
    }

    async fn read_public(path: &Path) -> Option<PublicKey> {
        let text = KeyText::read(path).await.ok()?;

        text.as_public_key()
    }
}
