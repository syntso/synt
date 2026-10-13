mod connections;
mod exec;
mod failure;
mod login;
mod privilege;

pub use connections::{ConnectionInfo, SshConnections};
pub use exec::{ExecParams, ExecResult, SshExec};
pub(crate) use failure::failure;
pub(crate) use login::{hidden_root, logs_in_as_root, resolve};
pub(crate) use privilege::is_root;
