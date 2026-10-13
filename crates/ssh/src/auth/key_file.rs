use std::io;
use std::path::{Path, PathBuf};

use russh::keys::{self, PrivateKey, PublicKey};
use tokio::io::AsyncReadExt;
use zeroize::Zeroizing;

const PUBLIC_SUFFIX: &str = ".pub";
const LARGEST_KEY_FILE: u64 = 256 * 1024;

pub(super) enum KeyFile {
    Missing,
    Encrypted,
    Unreadable,
    Unsupported,
    Loaded(Box<PrivateKey>),
}

impl KeyFile {
    pub(super) async fn read(path: &Path) -> io::Result<Self> {
        let text = match read_key_text(path).await {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::Missing),
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                return Ok(Self::Unreadable);
            }
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

    pub(super) async fn exists(path: &Path) -> bool {
        tokio::fs::try_exists(path).await.unwrap_or(false)
    }

    pub(super) async fn public_key(path: &Path) -> Option<PublicKey> {
        if let Some(key) = read_public(&public_path(path)).await {
            return Some(key);
        }

        let text = read_key_text(path).await.ok()?;
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
    let text = read_key_text(path).await.ok()?;

    PublicKey::from_openssh(text.trim()).ok()
}

async fn read_key_text(path: &Path) -> io::Result<Zeroizing<String>> {
    if !tokio::fs::metadata(path).await?.is_file() {
        return Err(not_a_key_file(path, "it is not a regular file"));
    }

    let file = tokio::fs::File::open(path).await?;
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(LARGEST_KEY_FILE + 1)
        .read_to_end(&mut bytes)
        .await?;

    if bytes.len() as u64 > LARGEST_KEY_FILE {
        return Err(not_a_key_file(path, "it is too large"));
    }

    let text = std::str::from_utf8(&bytes).map_err(|_| not_a_key_file(path, "it is not text"))?;

    Ok(Zeroizing::new(text.to_owned()))
}

fn not_a_key_file(path: &Path, reason: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{} cannot be a key file: {reason}", path.display()),
    )
}
