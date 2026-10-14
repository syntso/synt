use synt_proto::Actor;

use crate::error::JournalError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActorTag {
    Person = 1,
    Core = 2,
    Agent = 3,
    Runbook = 4,
    Plugin = 5,
}

impl ActorTag {
    pub(super) fn of(actor: &Actor) -> Self {
        match actor {
            Actor::Person => Self::Person,
            Actor::Core => Self::Core,
            Actor::Agent(_) => Self::Agent,
            Actor::Runbook(_) => Self::Runbook,
            Actor::Plugin(_) => Self::Plugin,
        }
    }
}

impl From<ActorTag> for u8 {
    fn from(tag: ActorTag) -> Self {
        tag as u8
    }
}

impl TryFrom<u8> for ActorTag {
    type Error = JournalError;

    fn try_from(code: u8) -> Result<Self, JournalError> {
        let tag = match code {
            1 => Self::Person,
            2 => Self::Core,
            3 => Self::Agent,
            4 => Self::Runbook,
            5 => Self::Plugin,
            _ => return Err(JournalError::Malformed("unknown actor")),
        };

        Ok(tag)
    }
}
