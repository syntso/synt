use super::Scope;
use crate::grants::GrantId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Decision {
    Allow,
    Approved(Scope),
    Granted(GrantId),
    Deny { reason: String },
}

impl Decision {
    pub(crate) fn grant(&self) -> Option<GrantId> {
        match self {
            Self::Granted(grant) => Some(*grant),
            Self::Allow | Self::Approved(_) | Self::Deny { .. } => None,
        }
    }

    pub(crate) fn deny(reason: impl Into<String>) -> Self {
        Self::Deny {
            reason: reason.into(),
        }
    }
}
