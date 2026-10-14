use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::ResolveParams;
use super::failure::{failure, invalid};
use crate::Syncer;
use crate::document::DocumentPath;
use crate::id::RevisionId;
use crate::report::RevisionInfo;

pub(crate) struct DocumentResolve {
    syncer: Syncer,
}

impl DocumentResolve {
    pub(crate) fn new(syncer: Syncer) -> Self {
        Self { syncer }
    }
}

impl Action for DocumentResolve {
    const ID: &'static str = "document.resolve";
    const TITLE: &'static str = "Settle a sync conflict by keeping one revision of a document";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = ResolveParams;
    type Output = RevisionInfo;

    fn summary(&self, output: &RevisionInfo) -> Value {
        json!({ "revision": output.revision, "parents": output.parents })
    }

    async fn run(
        &self,
        _context: &Context,
        params: ResolveParams,
    ) -> Result<RevisionInfo, ActionError> {
        let path = DocumentPath::parse(&params.path).map_err(invalid)?;
        let revision = RevisionId::parse(&params.revision).map_err(invalid)?;

        self.syncer
            .resolve(&params.project, path, revision)
            .await
            .map_err(failure)
    }
}
