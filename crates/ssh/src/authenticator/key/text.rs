use std::io;
use std::path::Path;
use std::str;
use std::sync::Arc;

use russh::keys::{self, PrivateKey, PublicKey};
use synt_platform::read_owned_file;
use tokio::task;
use zeroize::Zeroizing;

use crate::SshError;

const LARGEST_KEY_FILE: u64 = 256 * 1024;

pub(crate) struct KeyText {
    text: Zeroizing<String>,
}

impl KeyText {
    pub(crate) async fn read(path: &Path) -> Result<Self, SshError> {
        let owned = path.to_owned();

        let read = task::spawn_blocking(move || read_owned_file(&owned, LARGEST_KEY_FILE))
            .await
            .unwrap_or_else(|stopped| Err(io::Error::other(stopped)));

        let bytes = Zeroizing::new(read.map_err(|error| Self::refusal(path, error))?);

        let text = str::from_utf8(&bytes).map_err(|_| {
            let not_text = io::Error::new(io::ErrorKind::InvalidData, "it is not text");
            Self::refusal(path, not_text)
        })?;

        Ok(Self {
            text: Zeroizing::new(text.to_owned()),
        })
    }

    pub(crate) fn decode(&self) -> Result<PrivateKey, keys::Error> {
        keys::decode_secret_key(&self.text, None)
    }

    pub(in crate::authenticator) fn public_half(&self) -> Option<PublicKey> {
        let private = PrivateKey::from_openssh(self.text.as_bytes()).ok()?;

        Some(private.public_key().clone())
    }

    pub(in crate::authenticator) fn as_public_key(&self) -> Option<PublicKey> {
        PublicKey::from_openssh(self.text.trim()).ok()
    }

    fn refusal(path: &Path, error: io::Error) -> SshError {
        let path = path.to_owned();

        match error.kind() {
            io::ErrorKind::NotFound => SshError::NoKeyFile { path },
            io::ErrorKind::InvalidData => SshError::NotKeyFile {
                path,
                cause: Arc::new(error),
            },
            _ => SshError::UnreadableKeyFile {
                path,
                cause: Arc::new(error),
            },
        }
    }
}
