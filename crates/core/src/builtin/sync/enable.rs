use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_sync::{Place, StatusReport, Syncer};

use crate::projects::ProjectSlug;

pub struct SyncEnable {
    syncer: Syncer,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnableParams {
    pub(crate) project: ProjectSlug,
    pub folder: PathBuf,
}

impl SyncEnable {
    pub fn new(syncer: Syncer) -> Self {
        Self { syncer }
    }
}

impl Action for SyncEnable {
    const ID: &'static str = "sync.enable";
    const TITLE: &'static str = "Sync a project through a shared folder, sealed with its key";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = EnableParams;
    type Output = StatusReport;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &EnableParams,
    ) -> Result<(), ActionError> {
        if params.folder.is_absolute() {
            return Ok(());
        }

        Err(ActionError::InvalidParams(format!(
            "the sync folder must be an absolute path, not {}",
            params.folder.display()
        )))
    }

    fn summary(&self, output: &StatusReport) -> Value {
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
    ) -> Result<StatusReport, ActionError> {
        let place = Place::Folder(params.folder);

        Ok(self.syncer.enable(&params.project, place).await?)
    }
}
