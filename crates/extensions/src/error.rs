use std::io;
use std::time::Duration;

use synt_actions::ActionError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtensionError {
    #[error("extension files: {0}")]
    Io(#[from] io::Error),

    #[error("extension state: {0}")]
    State(#[from] rusqlite::Error),

    #[error("extension.toml: {0}")]
    Manifest(String),

    #[error("the component cannot be used: {0}")]
    Component(String),

    #[error("the extension failed: {0}")]
    Trapped(String),

    #[error("the extension ran out of fuel")]
    OutOfFuel,

    #[error("the extension ran past its time limit of {0:?}")]
    TimedOut(Duration),

    #[error("the extension made {0}")]
    OverQuota(String),

    #[error("the extension was stopped")]
    Stopped,

    #[error("the registry cannot be reached: {0}")]
    Fetch(String),

    #[error("the registry index is damaged: {0}")]
    Index(String),

    #[error("the signature does not check out: {0}")]
    Signature(String),

    #[error("no extension {0} is installed")]
    NotInstalled(String),

    #[error("the registry has no extension {0}")]
    NotListed(String),

    #[error("{0:?} is not a sha256 digest")]
    NotSha256(String),

    #[error("{0}")]
    Refused(String),
}

impl From<wasmtime::Error> for ExtensionError {
    fn from(error: wasmtime::Error) -> Self {
        Self::Component(format!("{error:#}"))
    }
}

impl From<toml::de::Error> for ExtensionError {
    fn from(error: toml::de::Error) -> Self {
        Self::Manifest(error.to_string())
    }
}

impl From<toml::ser::Error> for ExtensionError {
    fn from(error: toml::ser::Error) -> Self {
        Self::Manifest(error.to_string())
    }
}

impl From<minisign_verify::Error> for ExtensionError {
    fn from(error: minisign_verify::Error) -> Self {
        Self::Signature(error.to_string())
    }
}

impl From<ExtensionError> for ActionError {
    fn from(error: ExtensionError) -> Self {
        Self::failed(error.to_string())
    }
}
