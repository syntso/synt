mod directories;
mod endpoint;
mod error;
mod listener;
mod peer;
mod shutdown;
mod stream;
mod vault;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as native;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as native;

pub use directories::{create_private_directory, data_directory};
pub use endpoint::Endpoint;
pub use error::PlatformError;
pub use listener::LocalListener;
pub use peer::{AppCheck, Peer, UserId};
pub use shutdown::shutdown_signal;
pub use stream::LocalStream;
pub use vault::{KeyPlace, Vault, WEAK_VAULT_FLAG, WeakVault, open_vault};
