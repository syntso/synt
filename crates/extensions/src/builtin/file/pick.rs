use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::entry::FileEntry;
use crate::blocking::Blocking;
use crate::manifest::FileAccess;
use crate::picked::Picked;
use crate::{ExtensionError, Extensions};

#[derive(Clone)]
pub(crate) struct ExtensionFilePick {
    extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PickParams {
    pub extension: String,
    pub path: String,
    pub access: String,
}

impl ExtensionFilePick {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }

    fn pick(&self, params: &PickParams, wanted: FileAccess) -> Result<FileEntry, ExtensionError> {
        let loaded = self.extensions.loaded(&params.extension)?;
        let granted = loaded.guest.manifest().capabilities.files;

        if !granted.is_some_and(|granted| granted.covers(wanted)) {
            return Err(ExtensionError::Refused(format!(
                "{} may not {} files the person picks",
                params.extension,
                wanted.as_str()
            )));
        }

        let picked = Picked::pick(&params.path, wanted)?;
        self.extensions
            .services()
            .shelf
            .put_pick(&params.extension, &picked)?;

        Ok(FileEntry::new(&params.extension, picked))
    }
}

impl PickParams {
    fn access(&self) -> Result<FileAccess, ActionError> {
        FileAccess::parse(&self.access).ok_or_else(|| {
            ActionError::InvalidParams(format!(
                "access is read or read-write, not {:?}",
                self.access
            ))
        })
    }
}

impl Action for ExtensionFilePick {
    const ID: &'static str = "extension.file.pick";
    const TITLE: &'static str = "Give an extension one file the person picked";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = PickParams;
    type Output = FileEntry;

    fn check(&self, _target: Option<&ResourceRef>, params: &PickParams) -> Result<(), ActionError> {
        params.access().map(drop)
    }

    fn summary(&self, output: &FileEntry) -> Value {
        json!({
            "extension": output.extension,
            "handle": output.handle,
            "name": output.name,
            "access": output.access,
        })
    }

    async fn run(&self, _context: &Context, params: PickParams) -> Result<FileEntry, ActionError> {
        let wanted = params.access()?;
        let action = self.clone();

        Blocking::run(move || action.pick(&params, wanted)).await
    }
}
