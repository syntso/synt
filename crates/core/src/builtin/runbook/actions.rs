use synt_actions::{ActionError, Registry};

use super::{
    RunbookExport, RunbookFromBlocks, RunbookFromJournal, RunbookGet, RunbookList, RunbookPut,
    RunbookRun, RunbookStop,
};
use crate::runbooks::Runbooks;

#[derive(Debug, Clone)]
pub struct RunbookActions {
    runbooks: Runbooks,
}

impl RunbookActions {
    pub fn new(runbooks: Runbooks) -> Self {
        Self { runbooks }
    }

    pub fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(RunbookList::new(self.runbooks.clone()))?;
        registry.register(RunbookGet::new(self.runbooks.clone()))?;
        registry.register(RunbookPut::new(self.runbooks.clone()))?;
        registry.register(RunbookRun::new(self.runbooks.clone()))?;
        registry.register(RunbookStop::new(self.runbooks.clone()))?;
        registry.register(RunbookExport::new(self.runbooks.clone()))?;
        registry.register(RunbookFromJournal::new(self.runbooks.clone()))?;
        registry.register(RunbookFromBlocks::new(self.runbooks.clone()))
    }
}
