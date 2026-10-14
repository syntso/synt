use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::listed_document::ListedDocument;
use super::project_params::ProjectParams;
use crate::builtin::blocking::blocking;
use crate::documents::DocumentStore;
use crate::{ExtensionError, Extensions};

#[derive(Clone)]
pub(crate) struct ExtensionDocuments {
    extensions: Extensions,
}

impl ExtensionDocuments {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
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
        let extensions = self.extensions.clone();

        blocking(move || listed(&extensions, &params.project)).await
    }
}

fn listed(extensions: &Extensions, project: &str) -> Result<Vec<ListedDocument>, ExtensionError> {
    let services = extensions.services();
    let folders = services.folders.get().ok_or_else(|| {
        ExtensionError::Refused("the core has no projects to keep documents in".to_owned())
    })?;

    let mut documents = Vec::new();

    for loaded in extensions.all_loaded() {
        let manifest = loaded.guest.manifest();
        if !manifest.capabilities.documents {
            continue;
        }

        let store = DocumentStore::new(manifest, &services.shelf, folders.as_ref());
        let found = store.list(project).map_err(|error| {
            ExtensionError::Refused(format!("{}: {}", error.code, error.message))
        })?;

        documents.extend(found.into_iter().map(|info| ListedDocument {
            extension: manifest.id.clone(),
            info,
        }));
    }

    Ok(documents)
}
