use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum SshError {
    #[error("ssh config: {0}")]
    Config(String),

    #[error("cannot resolve {target:?}: {reason}")]
    Resolve { target: String, reason: String },

    #[error("cannot connect to {host}: {reason}")]
    Connect { host: String, reason: String },

    #[error("host key of {host} is not in known_hosts ({fingerprint})")]
    HostKeyUnknown { host: String, fingerprint: String },

    #[error("host key of {host} has changed ({fingerprint}); refusing to connect")]
    HostKeyChanged { host: String, fingerprint: String },

    #[error(
        "host key of {host} is of a type not pinned in known_hosts ({fingerprint}); \
         another type is pinned, so refusing to connect"
    )]
    HostKeyOtherType { host: String, fingerprint: String },

    #[error("{host} is not in known_hosts, so Synt does not connect to it ahead of time")]
    NotKnownHost { host: String },

    #[error("no accepted key, tried: {}", tried.join(", "))]
    NoAuthentication { tried: Vec<String> },

    #[error("key {} is encrypted; add it to ssh-agent with `ssh-add`", path.display())]
    EncryptedKey { path: PathBuf },

    #[error("{reason}")]
    KeyStore { reason: String },

    #[error("key secret://{name}: {reason}")]
    VaultKey { name: String, reason: String },

    #[error("channel: {0}")]
    Channel(String),

    #[error("timed out after {} s", .0.as_secs())]
    Timeout(Duration),

    #[error(
        "the command did not finish within {} s; {}",
        limit.as_secs(),
        remote_state(*ended)
    )]
    CommandTimeout { limit: Duration, ended: bool },

    #[error("the command was stopped before it finished; {}", remote_state(*ended))]
    Stopped { ended: bool },

    #[error("the call was stopped before the command started")]
    Cancelled,

    #[error("the connection was lost before the command finished")]
    ConnectionLost,

    #[error("the connection pool has shut down")]
    PoolClosed,

    #[error("sftp {operation} {path}: {reason}")]
    Sftp {
        operation: &'static str,
        path: String,
        reason: String,
    },

    #[error("sftp {operation} {path}: the sftp session was lost: {reason}")]
    SftpLost {
        operation: &'static str,
        path: String,
        reason: String,
    },

    #[error("the sftp session closed")]
    SftpClosed,

    #[error("cannot listen on 127.0.0.1:{port}: {reason}")]
    Listen { port: u16, reason: String },

    #[error("refused a local connection: {limit} connections are already open through the tunnel")]
    TunnelBusy { limit: usize },

    #[error(transparent)]
    Io(Arc<io::Error>),
}

impl From<io::Error> for SshError {
    fn from(error: io::Error) -> Self {
        Self::Io(Arc::new(error))
    }
}

impl From<russh::Error> for SshError {
    fn from(error: russh::Error) -> Self {
        Self::Channel(error.to_string())
    }
}

fn remote_state(ended: bool) -> &'static str {
    if ended {
        return "it was stopped with a signal";
    }

    "it was sent TERM and KILL, but may still be running if the server ignores signals"
}
