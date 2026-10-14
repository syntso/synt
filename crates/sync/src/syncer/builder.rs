use std::path::{Path, PathBuf};
use std::sync::Arc;

use synt_journal::JournalWriter;
use synt_platform::Vault;

use super::{Engine, ProjectFolders, Syncer};
use crate::SyncError;
use crate::history::History;
use crate::merge::DocumentMerger;

pub struct SyncerBuilder {
    state: PathBuf,
    vault: Arc<dyn Vault>,
    folders: Arc<dyn ProjectFolders>,
    journal: Option<JournalWriter>,
    merger: Option<Arc<dyn DocumentMerger>>,
}

impl SyncerBuilder {
    pub(crate) fn new(
        state: &Path,
        vault: Arc<dyn Vault>,
        folders: Arc<dyn ProjectFolders>,
    ) -> Self {
        Self {
            state: state.to_owned(),
            vault,
            folders,
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
        let engine = Engine::new(history, self.vault, self.folders, self.journal, self.merger);

        Ok(Syncer::new(engine))
    }
}
