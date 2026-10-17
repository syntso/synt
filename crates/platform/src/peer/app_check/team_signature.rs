use std::fmt;

use core_foundation::base::TCFType;
use core_foundation::data::CFData;
use security_framework::os::macos::code_signing::{
    Flags, GuestAttributes, SecCode, SecRequirement,
};

use super::Rule;
use crate::peer::AuditToken;
use crate::{Peer, PlatformError};

const TEAM: Option<&str> = option_env!("SYNT_TEAM_ID");

#[derive(Debug)]
pub(super) struct TeamSignature {
    team: String,
    requirement: String,
}

impl TeamSignature {
    pub(super) fn of_this_core() -> Option<Self> {
        let team = TEAM.filter(|team| Self::is_team_id(team))?;

        let signature = Self {
            team: team.to_owned(),
            requirement: format!(
                "anchor apple generic and certificate leaf[subject.OU] = \"{team}\""
            ),
        };

        signature.matches_this_core().then_some(signature)
    }

    fn matches_this_core(&self) -> bool {
        let Ok(requirement) = self.requirement() else {
            return false;
        };

        SecCode::for_self(Flags::NONE)
            .and_then(|code| code.check_validity(Flags::NONE, &requirement))
            .is_ok()
    }

    fn check_token(&self, audit_token: &AuditToken) -> Result<(), PlatformError> {
        let requirement = self.requirement()?;

        let token = CFData::from_buffer(audit_token);
        let mut attributes = GuestAttributes::new();
        attributes.set_audit_token(token.as_concrete_TypeRef());

        let code = SecCode::copy_guest_with_attribues(None, &attributes, Flags::NONE)
            .map_err(PlatformError::CodeNotFound)?;

        code.check_validity(Flags::NONE, &requirement)
            .map_err(|cause| PlatformError::NotSignedByTeam {
                team: self.team.clone(),
                cause,
            })
    }

    fn requirement(&self) -> Result<SecRequirement, PlatformError> {
        self.requirement
            .parse()
            .map_err(PlatformError::InvalidRequirement)
    }

    fn is_team_id(team: &str) -> bool {
        !team.is_empty()
            && team
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
    }
}

impl Rule for TeamSignature {
    fn weakness(&self) -> Option<&'static str> {
        None
    }

    fn check(&self, peer: &Peer) -> Result<(), PlatformError> {
        let token = peer.audit_token().ok_or(PlatformError::NoAuditToken)?;

        self.check_token(token)
    }
}

impl fmt::Display for TeamSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "they are validly signed by team {}", self.team)
    }
}
