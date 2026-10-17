mod export;
mod from;
mod get;
mod list;
mod params;
mod put;
mod run;
mod stop;

pub use export::RunbookExport;
pub use from::{RunbookFromBlocks, RunbookFromJournal};
pub use get::RunbookGet;
pub use list::RunbookList;
pub(crate) use params::RunbookParams;
pub use put::RunbookPut;
pub use run::RunbookRun;
pub use stop::RunbookStop;

use synt_actions::{ActionError, ActionGroup, Registry};

use crate::runbooks::Runbooks;

#[derive(Debug, Clone)]
pub struct RunbookActions {
    runbooks: Runbooks,
}

impl RunbookActions {
    pub fn new(runbooks: Runbooks) -> Self {
        Self { runbooks }
    }
}

impl ActionGroup for RunbookActions {
    fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
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
