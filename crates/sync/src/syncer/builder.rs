use std::path::{Path, PathBuf};
use std::sync::Arc;

use synt_journal::JournalWriter;
use synt_platform::Vault;

use super::Syncer;
use super::engine::Engine;
use crate::OpenProjects;
use crate::SyncError;
use crate::history::History;
use crate::merge::DocumentMerger;

pub struct SyncerBuilder {
    state: PathBuf,
    vault: Arc<dyn Vault>,
    projects: Arc<dyn OpenProjects>,
    journal: Option<JournalWriter>,
    merger: Option<Arc<dyn DocumentMerger>>,
}

impl SyncerBuilder {
    pub(crate) fn new(
        state: &Path,
        vault: Arc<dyn Vault>,
        projects: Arc<dyn OpenProjects>,
    ) -> Self {
        Self {
            state: state.to_owned(),
            vault,
            projects,
            journal: None,
            merger: None,
        }
    }

    #[must_use]
    pub fn journal(mut self, journal: JournalWriter) -> Self {
        self.journal = Some(journal);
        self
    }

    #[must_use]
    pub fn merger(mut self, merger: Arc<dyn DocumentMerger>) -> Self {
        self.merger = Some(merger);
        self
    }

    pub fn open(self) -> Result<Syncer, SyncError> {
        let history = History::open(&self.state)?;
        let engine = Engine::new(
            history,
            self.vault,
            self.projects,
            self.journal,
            self.merger,
        );

        Ok(Syncer::new(engine))
    }
}
