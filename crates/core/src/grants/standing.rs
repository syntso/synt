use synt_journal::Timestamp;

use super::{GrantId, GrantScope};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandingGrant {
    id: GrantId,
    scope: GrantScope,
    created_at: Timestamp,
}

impl StandingGrant {
    pub(crate) fn new(id: GrantId, scope: GrantScope, created_at: Timestamp) -> Self {
        Self {
            id,
            scope,
            created_at,
        }
    }

    pub(crate) fn id(&self) -> GrantId {
        self.id
    }

    pub(crate) fn scope(&self) -> &GrantScope {
        &self.scope
    }

    pub(crate) fn created_at(&self) -> Timestamp {
        self.created_at
    }
}
