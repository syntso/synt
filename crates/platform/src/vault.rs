#[cfg(windows)]
mod credentials;
#[cfg(unix)]
mod files;
mod key_place;
#[cfg(target_os = "macos")]
mod keychain;
mod weak;

use std::path::Path;

use zeroize::Zeroizing;

#[cfg(windows)]
use credentials::Credentials;
#[cfg(unix)]
use files::Files;
pub use key_place::KeyPlace;
#[cfg(target_os = "macos")]
use keychain::Keychain;
pub use weak::{WEAK_VAULT_FLAG, WeakVault};

use crate::PlatformError;

#[cfg(any(target_os = "macos", windows))]
const SERVICE: &str = "so.synt.core";

pub trait Vault: Send + Sync {
    fn place(&self) -> KeyPlace;

    fn read(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError>;

    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError>;

    fn remove(&self, name: &str) -> Result<(), PlatformError>;
}

pub fn open_vault(data: &Path, weak: WeakVault) -> Result<Box<dyn Vault>, PlatformError> {
    let vault = native_vault(data, weak)?;
    let place = vault.place();

    if place.weakness().is_some() && !weak.allowed() {
        return Err(PlatformError::WeakVault {
            place,
            flag: WEAK_VAULT_FLAG,
        });
    }

    Ok(vault)
}

#[cfg(target_os = "macos")]
fn native_vault(data: &Path, weak: WeakVault) -> Result<Box<dyn Vault>, PlatformError> {
    if cfg!(debug_assertions) {
        return Ok(Box::new(Files::open(data.join("keys"))?));
    }

    Ok(Box::new(Keychain::open(scope(data), weak)?))
}

#[cfg(windows)]
fn native_vault(data: &Path, _weak: WeakVault) -> Result<Box<dyn Vault>, PlatformError> {
    Ok(Box::new(Credentials::open(scope(data))?))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn native_vault(data: &Path, _weak: WeakVault) -> Result<Box<dyn Vault>, PlatformError> {
    Ok(Box::new(Files::open(data.join("keys"))?))
}

#[cfg(any(target_os = "macos", windows))]
fn scope(data: &Path) -> String {
    format!("{SERVICE}:{}", data.display())
}
