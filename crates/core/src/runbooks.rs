mod document;
mod draft;
mod error;
mod folder;
mod name;
mod param;
mod plan;
mod run;
mod shell;
mod template;

pub(crate) use document::OneLine;
pub use document::RunbookDocument;
pub(crate) use draft::{Draft, DraftStep};
pub use error::RunbookError;
pub(crate) use folder::RunbookFolder;
pub use folder::{RunbookEntry, RunbookListing};
pub use name::RunbookName;
pub(crate) use plan::Plan;
pub use run::{RunReport, Runs};
pub(crate) use shell::ShellScript;

use std::fmt::{self, Debug, Formatter};
use std::sync::{Arc, OnceLock, Weak};

use serde_json::{Map, Value};
use synt_actions::{Context, Risk};
use synt_journal::JournalWriter;

use crate::CoreError;
use crate::core::Core;
use crate::link::Link;
use crate::projects::Projects;
use crate::terminals::Terminals;
use run::{CoreSteps, LinkProgress, Run};

#[derive(Clone)]
pub struct Runbooks {
    projects: Projects,
    journal: JournalWriter,
    terminals: Terminals,
    runs: Runs,
    core: Arc<OnceLock<Weak<Core>>>,
}

impl Runbooks {
    pub fn new(projects: Projects, journal: JournalWriter, terminals: Terminals) -> Self {
        Self {
            projects,
            journal,
            terminals,
            runs: Runs::default(),
            core: Arc::default(),
        }
    }

    pub fn connect(&self, core: &Arc<Core>) -> Result<(), RunbookError> {
        self.core
            .set(Arc::downgrade(core))
            .map_err(|_| RunbookError::AlreadyConnected)
    }

    pub fn runs(&self) -> Runs {
        self.runs.clone()
    }

    pub(crate) fn folder(&self, project: &str) -> Result<RunbookFolder, CoreError> {
        let place = self.projects.place(project)?;

        Ok(RunbookFolder::new(place))
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
        name: &RunbookName,
        given: &Map<String, Value>,
    ) -> Result<Plan, CoreError> {
        let core = self.core()?;
        let folder = self.folder(project)?;
        let document = folder.read(name)?;

        let lookup = |id: &_| core.action(id);
        let plan = Plan::make(folder.reference(name), &document, given, &lookup)
            .map_err(|error| error.in_runbook(name))?;

        Ok(plan)
    }

    pub(crate) async fn run(
        &self,
        context: &Context,
        plan: &Plan,
        approve_steps: bool,
    ) -> Result<RunReport, RunbookError> {
        let core = self.core()?;
        let link = context
            .connection::<Link>()
            .ok_or(RunbookError::NoConnection)?;
        let runbook = plan.actor()?;

        let id = context.request();
        let guard = self.runs.start(id.get(), plan.reference(), context.stop());
        let stop = guard.stop().clone();

        let progress = LinkProgress::new(link.frames());
        let steps = CoreSteps::new(core, link, runbook, context, stop.clone())
            .approved(approve_steps && context.risk() >= Risk::Write);

        Ok(Run::new(steps, progress, stop, id.get())
            .perform(plan)
            .await)
    }

    pub(crate) fn stop(&self, run: u64) -> Result<String, RunbookError> {
        self.runs.stop(run)
    }

    fn core(&self) -> Result<Arc<Core>, RunbookError> {
        self.core
            .get()
            .and_then(Weak::upgrade)
            .ok_or(RunbookError::NotConnected)
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
