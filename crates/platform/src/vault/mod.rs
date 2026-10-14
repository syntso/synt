mod open;
mod place;
mod store;
mod weak;

#[cfg(target_os = "macos")]
mod keychain;

#[cfg(unix)]
mod files;

#[cfg(windows)]
mod credentials;

pub use open::open_vault;
pub use place::KeyPlace;
pub use store::Vault;
pub use weak::{WEAK_VAULT_FLAG, WeakVault};
