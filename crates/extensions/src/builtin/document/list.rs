use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::listed::ListedDocument;
use crate::builtin::blocking::blocking;
use crate::documents::DocumentStore;
use crate::{ExtensionError, Extensions};

#[derive(Clone)]
pub(crate) struct ExtensionDocuments {
    extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectParams {
    pub project: String,
}

impl ExtensionDocuments {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }

    fn listed(&self, project: &str) -> Result<Vec<ListedDocument>, ExtensionError> {
        let services = self.extensions.services();
        let folders = services.folders.get().ok_or_else(|| {
            ExtensionError::Refused("the core has no projects to keep documents in".to_owned())
        })?;

        let mut documents = Vec::new();

        for loaded in self.extensions.all_loaded() {
            let manifest = loaded.guest.manifest();
            if !manifest.capabilities.documents {
                continue;
            }

            let store = DocumentStore::new(manifest, &services.shelf, folders.as_ref());
            let found = store
                .list(project)
                .map_err(|error| ExtensionError::Refused(error.to_string()))?;

            documents.extend(
                found
                    .into_iter()
                    .map(|info| ListedDocument::new(&manifest.id, info)),
            );
        }

        Ok(documents)
    }
}

impl Action for ExtensionDocuments {
    const ID: &'static str = "extension.documents";
    const TITLE: &'static str = "List the documents extensions keep in a project";
    const RISK: Risk = Risk::Read;

    type Params = ProjectParams;
    type Output = Vec<ListedDocument>;

    fn summary(&self, output: &Vec<ListedDocument>) -> Value {
        json!({ "documents": output.len() })
    }

    async fn run(
        &self,
        _context: &Context,
        params: ProjectParams,
    ) -> Result<Vec<ListedDocument>, ActionError> {
        let action = self.clone();

        blocking(move || action.listed(&params.project)).await
    }
}
