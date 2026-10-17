use std::error::Error as StdError;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use russh_sftp::client::error::Error as SftpError;
use russh_sftp::protocol::StatusCode;
use ssh2_config::SshParserError;
use thiserror::Error;

type Cause = Arc<dyn StdError + Send + Sync>;

const SFTP_CLOSED: &str = "session closed";
const SFTP_NOT_SENT: &str = "SendError";

#[derive(Debug, Clone, Error)]
pub enum SshError {
    #[error("ssh config: {}: {cause}", .path.display())]
    Config {
        path: PathBuf,
        cause: Arc<SshParserError>,
    },

    #[error("ssh config: ProxyJump chain through {through:?} is longer than {limit} hops")]
    JumpChainTooLong { through: String, limit: usize },

    #[error("cannot resolve {target:?}: {reason}")]
    Resolve {
        target: String,
        reason: &'static str,
    },

    #[error("cannot connect to {host}: {cause}")]
    Connect { host: String, cause: Cause },

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

    #[error(
        "{} is protected by a passphrase; Synt cannot take passphrases yet, so decrypt a \
         copy first (ssh-keygen -p -N \"\" -f <copy>), import the copy and delete it",
        path.display()
    )]
    PassphraseKey { path: PathBuf },

    #[error(
        "{} is an RSA key; Synt keeps ed25519 and ecdsa keys, and RSA keys work \
         through ssh-agent only",
        path.display()
    )]
    RsaKey { path: PathBuf },

    #[error(
        "{} is not a private key Synt can read; give an OpenSSH or PEM private key file, \
         not a .pub file",
        path.display()
    )]
    NotPrivateKey { path: PathBuf },

    #[error("there is no file at {}", path.display())]
    NoKeyFile { path: PathBuf },

    #[error("{} cannot be a key file: {cause}", path.display())]
    NotKeyFile { path: PathBuf, cause: Arc<io::Error> },

    #[error("{} cannot be read: {cause}", path.display())]
    UnreadableKeyFile { path: PathBuf, cause: Arc<io::Error> },

    #[error("the system random source failed: {0}")]
    Random(getrandom::Error),

    #[error("the key could not be written in OpenSSH form")]
    KeyEncoding,

    #[error("key secret://{name}: {reason}")]
    VaultKey { name: String, reason: &'static str },

    #[error("key secret://{name}: {cause}")]
    VaultRead { name: String, cause: Cause },

    #[error("channel: {0}")]
    Protocol(Arc<russh::Error>),

    #[error("channel: the server refused {0}")]
    Refused(&'static str),

    #[error("channel: the channel closed before the server answered {0}")]
    Unanswered(&'static str),

    #[error("channel: sftp did not start: {0}")]
    SftpStart(SftpError),

    #[error("timed out after {} s", .0.as_secs())]
    Timeout(Duration),

    #[error(
        "the command did not finish within {} s; {}",
        limit.as_secs(),
        Self::remote_state(*ended)
    )]
    CommandTimeout { limit: Duration, ended: bool },

    #[error(
        "the command was stopped before it finished; {}",
        Self::remote_state(*ended)
    )]
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

    #[error("cannot listen on 127.0.0.1:{port}: {cause}")]
    Listen { port: u16, cause: Arc<io::Error> },

    #[error("refused a local connection: {limit} connections are already open through the tunnel")]
    TunnelBusy { limit: usize },

    #[error(transparent)]
    Io(Arc<io::Error>),
}

impl SshError {
    pub(crate) fn connect(host: String, cause: impl StdError + Send + Sync + 'static) -> Self {
        Self::Connect {
            host,
            cause: Arc::new(cause),
        }
    }

    pub(crate) fn sftp(operation: &'static str, path: &str, error: &SftpError) -> Self {
        let path = path.to_owned();

        match error {
            SftpError::Status(status) => Self::Sftp {
                operation,
                path,
                reason: Self::status_reason(&status.error_message, status.status_code),
            },
            SftpError::UnexpectedBehavior(message) if Self::is_unsent(message) => Self::SftpClosed,
            SftpError::Timeout => Self::SftpLost {
                operation,
                path,
                reason: "the server did not answer in time".to_owned(),
            },
            SftpError::IO(_) | SftpError::UnexpectedBehavior(_) => Self::SftpLost {
                operation,
                path,
                reason: error.to_string(),
            },
            SftpError::Limited(_) | SftpError::UnexpectedPacket => Self::Sftp {
                operation,
                path,
                reason: error.to_string(),
            },
        }
    }

    pub(crate) fn with_reason(self, reason: String) -> Self {
        match self {
            Self::Sftp {
                operation, path, ..
            } => Self::Sftp {
                operation,
                path,
                reason,
            },
            other => other,
        }
    }

    fn status_reason(message: &str, code: StatusCode) -> String {
        if message.is_empty() {
            return code.to_string();
        }

        message.to_owned()
    }

    fn is_unsent(message: &str) -> bool {
        message == SFTP_CLOSED || message.starts_with(SFTP_NOT_SENT)
    }

    fn remote_state(ended: bool) -> &'static str {
        if ended {
            return "it was stopped with a signal";
        }

        "it was sent TERM and KILL, but may still be running if the server ignores signals"
    }
}

impl From<io::Error> for SshError {
    fn from(error: io::Error) -> Self {
        Self::Io(Arc::new(error))
    }
}

impl From<russh::Error> for SshError {
    fn from(error: russh::Error) -> Self {
        Self::Protocol(Arc::new(error))
    }
}
