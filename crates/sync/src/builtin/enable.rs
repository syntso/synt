use std::path::PathBuf;

use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::EnableParams;
use super::failure::failure;
use crate::Syncer;
use crate::provider::Place;
use crate::report::SyncStatus;

pub(crate) struct SyncEnable {
    syncer: Syncer,
}

impl SyncEnable {
    pub(crate) fn new(syncer: Syncer) -> Self {
        Self { syncer }
    }
}

impl Action for SyncEnable {
    const ID: &'static str = "sync.enable";
    const TITLE: &'static str = "Sync a project through a shared folder, sealed with its key";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = EnableParams;
    type Output = SyncStatus;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &EnableParams,
    ) -> Result<(), ActionError> {
        if PathBuf::from(&params.folder).is_absolute() {
            return Ok(());
        }

        Err(ActionError::InvalidParams(format!(
            "the sync folder must be an absolute path, not {:?}",
            params.folder
        )))
    }

    fn summary(&self, output: &SyncStatus) -> Value {
        json!({
            "project": output.project,
            "access": output.access,
            "generation": output.generation,
        })
    }

    async fn run(
        &self,
        _context: &Context,
        params: EnableParams,
    ) -> Result<SyncStatus, ActionError> {
        let place = Place::Folder(PathBuf::from(params.folder));

        self.syncer
            .enable(&params.project, place)
            .await
            .map_err(failure)
    }
}
