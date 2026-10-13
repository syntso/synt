use std::fmt;

use tokio::task;

use super::{Peer, UserId};
use crate::{PlatformError, native};

#[cfg(target_os = "macos")]
use super::team_signature::TeamSignature;

#[cfg(target_os = "macos")]
const TRUSTED_BY_USER: &str = "this core is not signed with a team id, so apps are trusted by \
     user id only: any program running as this user can act as the app and answer approvals";

#[cfg(not(target_os = "macos"))]
const TRUSTED_BY_USER: &str = "apps are trusted by user id only: any program running as this \
     user can act as the app and answer approvals";

#[derive(Debug, Clone)]
pub struct AppCheck {
    rule: Rule,
}

#[derive(Debug, Clone)]
enum Rule {
    SameUser(UserId),
    #[cfg(target_os = "macos")]
    SignedBy(TeamSignature),
}

impl AppCheck {
    pub fn for_this_core() -> Result<Self, PlatformError> {
        #[cfg(target_os = "macos")]
        if let Some(signature) = TeamSignature::of_this_core() {
            return Ok(Self {
                rule: Rule::SignedBy(signature),
            });
        }

        #[cfg(unix)]
        let user = native::current_user();
        #[cfg(windows)]
        let user = native::current_user()?;

        Ok(Self {
            rule: Rule::SameUser(user),
        })
    }

    pub fn weakness(&self) -> Option<&'static str> {
        match self.rule {
            Rule::SameUser(_) => Some(TRUSTED_BY_USER),
            #[cfg(target_os = "macos")]
            Rule::SignedBy(_) => None,
        }
    }

    pub async fn verify(&self, peer: Peer) -> Result<(), PlatformError> {
        let rule = self.rule.clone();
        let described = peer.to_string();

        let verdict = task::spawn_blocking(move || rule.check(&peer))
            .await
            .unwrap_or_else(|error| Err(format!("the check stopped: {error}")));

        verdict.map_err(|reason| PlatformError::UnverifiedApp {
            peer: described,
            reason,
        })
    }
}

impl Rule {
    fn check(&self, peer: &Peer) -> Result<(), String> {
        match self {
            Self::SameUser(user) => same_user(user, peer),
            #[cfg(target_os = "macos")]
            Self::SignedBy(signature) => {
                let token = peer
                    .audit_token()
                    .ok_or("the system gave no audit token for it")?;
                signature.check(token)
            }
        }
    }
}

fn same_user(core: &UserId, peer: &Peer) -> Result<(), String> {
    match peer.user() {
        Some(user) if user == core => Ok(()),
        Some(user) => Err(format!("it runs as user {user}, the core as user {core}")),
        None => Err("the system did not say which user it runs as".to_owned()),
    }
}

impl fmt::Display for AppCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.rule {
            Rule::SameUser(user) => write!(f, "they run as user {user}"),
            #[cfg(target_os = "macos")]
            Rule::SignedBy(signature) => {
                write!(f, "they are validly signed by team {}", signature.team())
            }
        }
    }
}
