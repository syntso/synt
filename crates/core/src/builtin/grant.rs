mod info;
mod list;
mod revoke;

pub(crate) use info::GrantInfo;
pub use list::GrantList;
pub use revoke::GrantRevoke;

use synt_actions::{ActionError, ActionGroup, Registry};
use synt_journal::JournalWriter;

use crate::grants::AlwaysGrants;

#[derive(Clone)]
pub(crate) struct GrantActions {
    grants: AlwaysGrants,
    journal: JournalWriter,
}

impl GrantActions {
    pub fn new(grants: AlwaysGrants, journal: JournalWriter) -> Self {
        Self { grants, journal }
    }
}

impl ActionGroup for GrantActions {
    fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(GrantList::new(self.grants.clone()))?;
        registry.register(GrantRevoke::new(self.grants.clone(), self.journal.clone()))
    }
}
