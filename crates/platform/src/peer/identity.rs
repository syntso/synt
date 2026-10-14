use std::fmt;

use super::UserId;

#[cfg(target_os = "macos")]
pub(crate) type AuditToken = [u8; 32];

#[derive(Debug, Clone)]
pub struct Peer {
    user: Option<UserId>,
    process: Option<u32>,
    #[cfg(target_os = "macos")]
    audit_token: Option<AuditToken>,
}

impl Peer {
    pub(crate) fn new(user: Option<UserId>, process: Option<u32>) -> Self {
        Self {
            user,
            process,
            #[cfg(target_os = "macos")]
            audit_token: None,
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn with_audit_token(self, audit_token: Option<AuditToken>) -> Self {
        Self {
            audit_token,
            ..self
        }
    }

    pub fn user(&self) -> Option<&UserId> {
        self.user.as_ref()
    }

    pub fn process(&self) -> Option<u32> {
        self.process
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn audit_token(&self) -> Option<&AuditToken> {
        self.audit_token.as_ref()
    }
}

impl fmt::Display for Peer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.process, &self.user) {
            (Some(process), Some(user)) => write!(f, "process {process} of user {user}"),
            (Some(process), None) => write!(f, "process {process}"),
            (None, Some(user)) => write!(f, "a process of user {user}"),
            (None, None) => f.write_str("an unknown process"),
        }
    }
}
