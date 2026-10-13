use std::io;
use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Error)]
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

    #[error("no accepted key, tried: {}", tried.join(", "))]
    NoAuthentication { tried: Vec<String> },

    #[error("key {} is encrypted; add it to ssh-agent with `ssh-add`", path.display())]
    EncryptedKey { path: PathBuf },

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

    #[error(transparent)]
    Io(#[from] io::Error),
}

fn remote_state(ended: bool) -> &'static str {
    if ended {
        return "it was stopped with a signal";
    }

    "it was sent TERM and KILL, but may still be running if the server ignores signals"
}
