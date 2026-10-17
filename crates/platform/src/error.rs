use std::error::Error as StdError;
use std::io;
use std::path::PathBuf;

use thiserror::Error;
use tokio::task::JoinError;

use crate::{KeyPlace, UserId};

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("a {noun} name must be 1 to {longest} characters")]
    VaultNameLength { noun: &'static str, longest: usize },

    #[error(
        "{text:?} is not a valid {noun} name: use letters, digits, '-' and '_', \
         with '/' between parts"
    )]
    InvalidVaultName { noun: &'static str, text: String },

    #[error("{0}")]
    Io(#[from] io::Error),

    #[cfg(target_os = "macos")]
    #[error("keychain: {0}")]
    Keychain(#[from] security_framework::base::Error),

    #[cfg(windows)]
    #[error("credential manager: {0}")]
    Credentials(#[from] keyring_core::Error),

    #[error(
        "the journal key would be kept in {place}, and {}; a release build refuses that \
         unless the core is started with {flag}, which is meant for development only",
        .place.weakness().unwrap_or("it is not protected")
    )]
    WeakVault { place: KeyPlace, flag: &'static str },

    #[error("terminal: {0}")]
    Terminal(Box<dyn StdError + Send + Sync>),

    #[error("terminal: the shell no longer takes input")]
    ShellGone,

    #[error("terminal: the terminal is closed")]
    TerminalClosed,

    #[error("{}: {reason}", .tool.display())]
    Tool { tool: PathBuf, reason: String },

    #[error("{} is not registered to start at login: {reason}", .program.display())]
    ProgramRefused {
        program: PathBuf,
        reason: &'static str,
    },

    #[error(
        "{} is not registered to start at login: {} can be changed by other accounts",
        .program.display(),
        .place.display()
    )]
    ProgramExposed { program: PathBuf, place: PathBuf },

    #[error("the core's path is not valid Unicode, so a {registration} cannot name it")]
    NotUnicode { registration: &'static str },

    #[error("{peer} says it is the app, but it is not verified: {reason}")]
    UnverifiedApp {
        peer: String,
        reason: Box<PlatformError>,
    },

    #[error("it runs as user {peer}, the core as user {core}")]
    OtherUser { peer: UserId, core: UserId },

    #[error("the system did not say which user it runs as")]
    UnknownUser,

    #[error("the check stopped: {0}")]
    CheckStopped(JoinError),

    #[cfg(target_os = "macos")]
    #[error("the system gave no audit token for it")]
    NoAuditToken,

    #[cfg(target_os = "macos")]
    #[error("its code cannot be found: {0}")]
    CodeNotFound(security_framework::base::Error),

    #[cfg(target_os = "macos")]
    #[error("it is not validly signed by team {team}: {cause}")]
    NotSignedByTeam {
        team: String,
        cause: security_framework::base::Error,
    },

    #[cfg(target_os = "macos")]
    #[error("the signing requirement is invalid: {0}")]
    InvalidRequirement(security_framework::base::Error),
}

impl PlatformError {
    pub(crate) fn terminal(error: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self::Terminal(error.into())
    }
}
