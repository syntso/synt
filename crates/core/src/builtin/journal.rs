mod page;
mod read;
mod verify;

pub(crate) use page::JournalPage;
pub(crate) use verify::JournalVerify;

use synt_actions::{ActionError, ActionGroup, Registry};
use synt_journal::JournalWriter;

#[derive(Clone)]
pub(crate) struct JournalActions {
    journal: JournalWriter,
}

impl JournalActions {
    pub fn new(journal: JournalWriter) -> Self {
        Self { journal }
    }
}

impl ActionGroup for JournalActions {
    fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(JournalPage::new(self.journal.clone()))?;
        registry.register(JournalVerify::new(self.journal.clone()))
    }
}
