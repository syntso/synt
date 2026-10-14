use std::path::Path;

use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::file_entry::FileEntry;
use super::pick_params::PickParams;
use crate::builtin::blocking::blocking;
use crate::files::{FileGate, Picked};
use crate::manifest::FileAccess;
use crate::{ExtensionError, Extensions};

#[derive(Clone)]
pub(crate) struct ExtensionFilePick {
    extensions: Extensions,
}

impl ExtensionFilePick {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
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
        access(&params.access).map(drop)
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
        let wanted = access(&params.access)?;
        let extensions = self.extensions.clone();

        blocking(move || pick(&extensions, &params, wanted)).await
    }
}

fn pick(
    extensions: &Extensions,
    params: &PickParams,
    wanted: FileAccess,
) -> Result<FileEntry, ExtensionError> {
    let loaded = extensions.loaded(&params.extension)?;
    let granted = loaded.guest.manifest().capabilities.files;

    if !granted.is_some_and(|granted| granted.covers(wanted)) {
        return Err(ExtensionError::Refused(format!(
            "{} may not {} files the person picks",
            params.extension,
            wanted.as_str()
        )));
    }

    FileGate::check_pick(Path::new(&params.path), wanted).map_err(ExtensionError::Refused)?;

    let picked = Picked {
        handle: new_handle()?,
        name: Picked::name_of(&params.path),
        access: wanted,
        path: params.path.clone(),
    };
    extensions
        .services()
        .shelf
        .put_pick(&params.extension, &picked)?;

    Ok(FileEntry::new(&params.extension, picked))
}

fn access(text: &str) -> Result<FileAccess, ActionError> {
    FileAccess::parse(text).ok_or_else(|| {
        ActionError::InvalidParams(format!("access is read or read-write, not {text:?}"))
    })
}

fn new_handle() -> Result<String, ExtensionError> {
    let mut bytes = [0_u8; 12];
    getrandom::fill(&mut bytes)
        .map_err(|error| ExtensionError::Refused(format!("system random source: {error}")))?;

    Ok(format!("file-{}", data_encoding::HEXLOWER.encode(&bytes)))
}
