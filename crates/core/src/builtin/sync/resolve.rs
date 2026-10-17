use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_sync::{DocumentPath, RevisionId, RevisionInfo, Syncer};

use crate::projects::ProjectSlug;

pub struct DocumentResolve {
    syncer: Syncer,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolveParams {
    pub(crate) project: ProjectSlug,
    #[schemars(with = "String")]
    pub path: DocumentPath,
    #[schemars(with = "String")]
    pub revision: RevisionId,
}

impl DocumentResolve {
    pub fn new(syncer: Syncer) -> Self {
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
        let resolved = self
            .syncer
            .resolve(&params.project, params.path, params.revision)
            .await?;

        Ok(resolved)
    }
}
