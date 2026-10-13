use std::io;

use thiserror::Error;

use crate::KeyPlace;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("{0}")]
    Io(#[from] io::Error),

    #[cfg(target_os = "macos")]
    #[error("keychain: {0}")]
    Keychain(#[from] security_framework::base::Error),

    #[cfg(windows)]
    #[error("credential manager: {0}")]
    Credentials(String),

    #[error(
        "the journal key would be kept in {place}, and {}; a release build refuses that \
         unless the core is started with {flag}, which is meant for development only",
        .place.weakness().unwrap_or("it is not protected")
    )]
    WeakVault { place: KeyPlace, flag: &'static str },

    #[error("{tool}: {reason}")]
    Tool { tool: String, reason: String },

    #[error("{peer} says it is the app, but it is not verified: {reason}")]
    UnverifiedApp { peer: String, reason: String },
}
