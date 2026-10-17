use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk, blocking};

use super::entry::FileEntry;
use crate::ExtensionError;
use crate::Extensions;
use crate::builtin::extension_params::ExtensionParams;

#[derive(Clone)]
pub(crate) struct ExtensionFileList {
    extensions: Extensions,
}

impl ExtensionFileList {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionFileList {
    const ID: &'static str = "extension.file.list";
    const TITLE: &'static str = "List the files the person picked for an extension";
    const RISK: Risk = Risk::Read;
    const PERSON_ONLY: bool = true;

    type Params = ExtensionParams;
    type Output = Vec<FileEntry>;

    fn summary(&self, output: &Vec<FileEntry>) -> Value {
        json!({ "files": output.len() })
    }

    async fn run(
        &self,
        _context: &Context,
        params: ExtensionParams,
    ) -> Result<Vec<FileEntry>, ActionError> {
        let extensions = self.extensions.clone();

        blocking(move || -> Result<_, ExtensionError> {
            let extension = params.extension();
            let picks = extensions.services().shelf().picks().all(extension)?;

            Ok(picks
                .iter()
                .map(|picked| FileEntry::new(extension, picked))
                .collect())
        })
        .await
    }
}
