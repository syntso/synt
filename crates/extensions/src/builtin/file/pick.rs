use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};

use super::entry::FileEntry;
use crate::blocking::Blocking;
use crate::files::{FileAccess, Picked};
use crate::{ExtensionError, Extensions};

#[derive(Clone)]
pub(crate) struct ExtensionFilePick {
    extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PickParams {
    extension: String,
    path: String,
    access: FileAccess,
}

impl ExtensionFilePick {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }

    fn pick(&self, params: &PickParams) -> Result<FileEntry, ExtensionError> {
        let loaded = self.extensions.loaded(&params.extension)?;
        let granted = loaded.guest().manifest().capabilities().files();

        if !granted.is_some_and(|granted| granted.covers(params.access)) {
            return Err(ExtensionError::FilesNotGranted {
                extension: params.extension.clone(),
                access: params.access,
            });
        }

        let picked = Picked::pick(&params.path, params.access)?;
        let services = self.extensions.services();

        services
            .shelf()
            .picks()
            .put(&params.extension, &picked, services.now())?;

        Ok(FileEntry::new(&params.extension, &picked))
    }
}

impl Action for ExtensionFilePick {
    const ID: &'static str = "extension.file.pick";
    const TITLE: &'static str = "Give an extension one file the person picked";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = PickParams;
    type Output = FileEntry;

    fn summary(&self, output: &FileEntry) -> Value {
        output.summary()
    }

    async fn run(&self, _context: &Context, params: PickParams) -> Result<FileEntry, ActionError> {
        let action = self.clone();

        Blocking::run(move || action.pick(&params)).await
    }
}
