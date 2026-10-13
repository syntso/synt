mod open;
mod place;
mod vault;
mod weak_vault;

#[cfg(target_os = "macos")]
mod keychain;

#[cfg(unix)]
mod files;

#[cfg(windows)]
mod credentials;

pub use open::open_vault;
pub use place::KeyPlace;
pub use vault::Vault;
pub use weak_vault::{WEAK_VAULT_FLAG, WeakVault};
