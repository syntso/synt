mod connections;
mod exec;
mod failure;
mod login;
mod privilege;
mod warm;

pub use connections::{ConnectionInfo, SshConnections};
pub use exec::{ExecParams, ExecResult, SshExec};
pub(crate) use failure::failure;
pub(crate) use login::{check_reach, destination, logs_in_as_root, reach_host, resolve_any};
pub(crate) use privilege::is_root;
pub use warm::{SshWarm, Warmed};
