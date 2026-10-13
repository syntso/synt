mod account;
mod detached;
mod directories;
mod endpoint;
mod error;
mod files;
mod listener;
mod peer;
mod service;
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

pub use account::{home_directory, user_name};
pub use detached::start_detached;
pub use directories::{create_private_directory, data_directory};
pub use endpoint::Endpoint;
pub use error::PlatformError;
pub use files::{read_plain_file, replace_file};
pub use listener::LocalListener;
pub use peer::{AppCheck, Peer, UserId};
pub use service::{LoginStart, LoginStatus};
pub use shutdown::shutdown_signal;
pub use stream::LocalStream;
pub use vault::{KeyPlace, Vault, WEAK_VAULT_FLAG, WeakVault, open_vault};
