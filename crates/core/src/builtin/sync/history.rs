use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_sync::{DocumentPath, RevisionInfo, Syncer};

use crate::projects::ProjectSlug;

pub struct DocumentHistory {
    syncer: Syncer,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HistoryParams {
    pub(crate) project: ProjectSlug,
    #[schemars(with = "String")]
    pub path: DocumentPath,
}

impl DocumentHistory {
    pub fn new(syncer: Syncer) -> Self {
        Self { syncer }
    }
}

impl Action for DocumentHistory {
    const ID: &'static str = "document.history";
    const TITLE: &'static str = "List the synced revisions of a project document";
    const RISK: Risk = Risk::Read;

    type Params = HistoryParams;
    type Output = Vec<RevisionInfo>;

    fn summary(&self, output: &Vec<RevisionInfo>) -> Value {
        json!({ "revisions": output.len() })
    }

    async fn run(
        &self,
        _context: &Context,
        params: HistoryParams,
    ) -> Result<Vec<RevisionInfo>, ActionError> {
        Ok(self.syncer.history(&params.project, params.path).await?)
    }
}
