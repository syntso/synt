use std::fmt::{self, Debug, Formatter};
use std::sync::{Arc, OnceLock, Weak};

use serde_json::{Map, Value};
use synt_actions::{ActionError, Context, Registry, Risk};
use synt_journal::JournalWriter;

use super::plan::Plan;
use super::run::{CoreSteps, LinkProgress, RunReport, Runner, Runs};
use super::store::RunbookStore;
use crate::CoreError;
use crate::builtin::{
    RunbookExport, RunbookFromBlocks, RunbookFromJournal, RunbookGet, RunbookList, RunbookPut,
    RunbookRun, RunbookStop,
};
use crate::core::Core;
use crate::link::Link;
use crate::projects::Projects;
use crate::terminals::Terminals;

#[derive(Clone)]
pub struct Runbooks {
    store: RunbookStore,
    journal: JournalWriter,
    terminals: Terminals,
    runs: Runs,
    core: Arc<OnceLock<Weak<Core>>>,
}

impl Runbooks {
    pub fn new(projects: Projects, journal: JournalWriter, terminals: Terminals) -> Self {
        Self {
            store: RunbookStore::new(projects),
            journal,
            terminals,
            runs: Runs::default(),
            core: Arc::default(),
        }
    }

    pub fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(RunbookList::new(self.clone()))?;
        registry.register(RunbookGet::new(self.clone()))?;
        registry.register(RunbookPut::new(self.clone()))?;
        registry.register(RunbookRun::new(self.clone()))?;
        registry.register(RunbookStop::new(self.clone()))?;
        registry.register(RunbookExport::new(self.clone()))?;
        registry.register(RunbookFromJournal::new(self.clone()))?;
        registry.register(RunbookFromBlocks::new(self.clone()))
    }

    pub fn connect(&self, core: &Arc<Core>) {
        let _ = self.core.set(Arc::downgrade(core));
    }

    pub fn runs(&self) -> Runs {
        self.runs.clone()
    }

    pub(crate) fn store(&self) -> &RunbookStore {
        &self.store
    }

    pub(crate) fn journal(&self) -> &JournalWriter {
        &self.journal
    }

    pub(crate) fn terminals(&self) -> &Terminals {
        &self.terminals
    }

    pub(crate) fn plan(
        &self,
        project: &str,
        name: &str,
        given: &Map<String, Value>,
    ) -> Result<Plan, CoreError> {
        let core = self
            .core()
            .map_err(|error| CoreError::Runbook(error.to_string()))?;
        let (reference, runbook) = self.store.get(project, name)?;
        let lookup = |id: &_| core.action(id);

        Plan::make(reference, &runbook, given, &lookup)
            .map_err(|reason| CoreError::Runbook(format!("runbook {name}: {reason}")))
    }

    pub(crate) async fn run(
        &self,
        context: &Context,
        plan: &Plan,
        approve_steps: bool,
    ) -> Result<RunReport, ActionError> {
        let core = self.core()?;
        let link = context.connection::<Link>().ok_or_else(|| {
            ActionError::failed("this call has no connection to report progress on")
        })?;
        let runbook = plan.actor().map_err(ActionError::failed)?;

        let run = context.request();
        let guard = self.runs.start(run.get(), &plan.runbook, context.stop());

        let steps = CoreSteps {
            core,
            link: Arc::clone(&link),
            runbook,
            starter: context.actor().clone(),
            run,
            approved: approve_steps && context.risk() >= Risk::Write,
            stop: guard.stop().clone(),
        };
        let progress = LinkProgress::new(link.frames());

        let runner = Runner::new(steps, progress, guard.stop().clone(), run.get());

        Ok(runner.run(plan).await)
    }

    pub(crate) fn stop(&self, run: u64) -> Option<String> {
        self.runs.stop(run)
    }

    fn core(&self) -> Result<Arc<Core>, ActionError> {
        self.core
            .get()
            .and_then(Weak::upgrade)
            .ok_or_else(|| ActionError::failed("runbooks are not connected to a running core"))
    }
}

impl Debug for Runbooks {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Runbooks")
            .field("runs", &self.runs)
            .finish_non_exhaustive()
    }
}
