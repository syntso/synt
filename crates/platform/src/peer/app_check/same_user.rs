use std::fmt;

use super::Rule;
use crate::{Peer, PlatformError, UserId};

#[cfg(target_os = "macos")]
const TRUSTED_BY_USER: &str = "this core is not signed with a team id, so apps are trusted by \
     user id only: any program running as this user can act as the app and answer approvals";

#[cfg(not(target_os = "macos"))]
const TRUSTED_BY_USER: &str = "apps are trusted by user id only: any program running as this \
     user can act as the app and answer approvals";

#[derive(Debug)]
pub(super) struct SameUser {
    core: UserId,
}

impl SameUser {
    pub(super) fn new(core: UserId) -> Self {
        Self { core }
    }
}

impl Rule for SameUser {
    fn weakness(&self) -> Option<&'static str> {
        Some(TRUSTED_BY_USER)
    }

    fn check(&self, peer: &Peer) -> Result<(), PlatformError> {
        let Some(user) = peer.user() else {
            return Err(PlatformError::UnknownUser);
        };

        if *user != self.core {
            return Err(PlatformError::OtherUser {
                peer: user.clone(),
                core: self.core.clone(),
            });
        }

        Ok(())
    }
}

impl fmt::Display for SameUser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "they run as user {}", self.core)
    }
}
