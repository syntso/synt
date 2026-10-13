use core_foundation::base::TCFType;
use core_foundation::data::CFData;
use security_framework::os::macos::code_signing::{
    Flags, GuestAttributes, SecCode, SecRequirement,
};

use super::AuditToken;

const TEAM: Option<&str> = option_env!("SYNT_TEAM_ID");

#[derive(Debug, Clone)]
pub(crate) struct TeamSignature {
    team: String,
    requirement: String,
}

impl TeamSignature {
    pub(crate) fn of_this_core() -> Option<Self> {
        let team = TEAM.filter(|team| is_team_id(team))?;

        let signature = Self {
            team: team.to_owned(),
            requirement: format!(
                "anchor apple generic and certificate leaf[subject.OU] = \"{team}\""
            ),
        };

        signature.matches_this_core().then_some(signature)
    }

    pub(crate) fn team(&self) -> &str {
        &self.team
    }

    pub(crate) fn check(&self, audit_token: &AuditToken) -> Result<(), String> {
        let requirement = self.requirement()?;

        let token = CFData::from_buffer(audit_token);
        let mut attributes = GuestAttributes::new();
        attributes.set_audit_token(token.as_concrete_TypeRef());

        let code = SecCode::copy_guest_with_attribues(None, &attributes, Flags::NONE)
            .map_err(|error| format!("its code cannot be found: {error}"))?;

        code.check_validity(Flags::NONE, &requirement)
            .map_err(|error| format!("it is not validly signed by team {}: {error}", self.team))
    }

    fn matches_this_core(&self) -> bool {
        let Ok(requirement) = self.requirement() else {
            return false;
        };

        SecCode::for_self(Flags::NONE)
            .and_then(|code| code.check_validity(Flags::NONE, &requirement))
            .is_ok()
    }

    fn requirement(&self) -> Result<SecRequirement, String> {
        self.requirement
            .parse()
            .map_err(|error| format!("the signing requirement is invalid: {error}"))
    }
}

fn is_team_id(team: &str) -> bool {
    !team.is_empty()
        && team
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}
