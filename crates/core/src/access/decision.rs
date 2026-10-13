use super::Scope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Approved(Scope),
    Deny { reason: String },
}

impl Decision {
    pub fn deny(reason: impl Into<String>) -> Self {
        Self::Deny {
            reason: reason.into(),
        }
    }
}
