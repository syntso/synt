mod account;
mod detached;
mod directories;
mod endpoint;
mod error;
mod files;
#[cfg(fuzzing)]
pub mod fuzzing;
mod local;
mod lock;
mod login;
mod peer;
mod pty;
mod shutdown;
mod vault;

pub use account::{home_directory, user_name};
pub use detached::start_detached;
pub use directories::{create_private_directory, data_directory};
pub use endpoint::Endpoint;
pub use error::PlatformError;
pub use files::{Found, read_owned_file, read_plain_file, replace_file};
pub use local::{LocalListener, LocalStream};
pub use lock::Lock;
pub use login::{LoginStart, LoginStatus};
pub use peer::{AppCheck, Peer, UserId};
pub use pty::{Pty, PtyEnd, PtyOutput};
pub use shutdown::shutdown_signal;
pub use vault::{
    KeyPlace, SecretItem, SecretName, Vault, VaultKind, VaultName, WEAK_VAULT_FLAG, WeakVault,
    open_vault,
};
