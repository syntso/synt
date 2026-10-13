use std::path::Path;

use super::vault::Vault;
use super::weak_vault::{WEAK_VAULT_FLAG, WeakVault};
use crate::PlatformError;

#[cfg(any(target_os = "macos", windows))]
const SERVICE: &str = "so.synt.core";

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
        let files = super::files::Files::open(data.join("keys"))?;
        return Ok(Box::new(files));
    }

    let keychain = super::keychain::Keychain::open(scope(data), weak)?;
    Ok(Box::new(keychain))
}

#[cfg(windows)]
fn native_vault(data: &Path, _weak: WeakVault) -> Result<Box<dyn Vault>, PlatformError> {
    let credentials = super::credentials::Credentials::open(scope(data))?;
    Ok(Box::new(credentials))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn native_vault(data: &Path, _weak: WeakVault) -> Result<Box<dyn Vault>, PlatformError> {
    let files = super::files::Files::open(data.join("keys"))?;
    Ok(Box::new(files))
}

#[cfg(any(target_os = "macos", windows))]
fn scope(data: &Path) -> String {
    format!("{SERVICE}:{}", data.display())
}
