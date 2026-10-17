mod rule;
mod same_user;
#[cfg(target_os = "macos")]
mod team_signature;

use std::fmt;
use std::sync::Arc;

use tokio::task;

use rule::Rule;
use same_user::SameUser;
#[cfg(target_os = "macos")]
use team_signature::TeamSignature;

use super::{Peer, UserId};
use crate::PlatformError;

#[derive(Debug, Clone)]
pub struct AppCheck {
    rule: Arc<dyn Rule>,
}

impl AppCheck {
    pub fn for_this_core() -> Result<Self, PlatformError> {
        #[cfg(target_os = "macos")]
        if let Some(signature) = TeamSignature::of_this_core() {
            return Ok(Self {
                rule: Arc::new(signature),
            });
        }

        let user = SameUser::new(UserId::current()?);

        Ok(Self {
            rule: Arc::new(user),
        })
    }

    pub fn weakness(&self) -> Option<&'static str> {
        self.rule.weakness()
    }

    pub async fn verify(&self, peer: Peer) -> Result<(), PlatformError> {
        let rule = Arc::clone(&self.rule);
        let described = peer.to_string();

        let verdict = task::spawn_blocking(move || rule.check(&peer))
            .await
            .unwrap_or_else(|stopped| Err(PlatformError::CheckStopped(stopped)));

        verdict.map_err(|reason| PlatformError::UnverifiedApp {
            peer: described,
            reason: Box::new(reason),
        })
    }
}

impl fmt::Display for AppCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.rule, f)
    }
}
