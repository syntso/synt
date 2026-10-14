use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::params::PathParams;
use crate::Syncer;
use crate::document::DocumentPath;
use crate::report::RevisionInfo;

pub(crate) struct DocumentHistory {
    syncer: Syncer,
}

impl DocumentHistory {
    pub(crate) fn new(syncer: Syncer) -> Self {
        Self { syncer }
    }
}

impl Action for DocumentHistory {
    const ID: &'static str = "document.history";
    const TITLE: &'static str = "List the synced revisions of a project document";
    const RISK: Risk = Risk::Read;

    type Params = PathParams;
    type Output = Vec<RevisionInfo>;

    fn summary(&self, output: &Vec<RevisionInfo>) -> Value {
        json!({ "revisions": output.len() })
    }

    async fn run(
        &self,
        _context: &Context,
        params: PathParams,
    ) -> Result<Vec<RevisionInfo>, ActionError> {
        let path = DocumentPath::parse(&params.path)?;

        Ok(self.syncer.history(&params.project, path).await?)
    }
}
