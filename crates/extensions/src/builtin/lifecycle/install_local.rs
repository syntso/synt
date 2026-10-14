use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::ExtensionInfo;
use crate::Extensions;
use crate::builtin::blocking::blocking;
use crate::extension::Candidate;

#[derive(Clone)]
pub(crate) struct ExtensionInstallLocal {
    extensions: Extensions,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct LocalParams {
    pub path: String,

    #[serde(skip)]
    #[schemars(skip)]
    candidate: Option<Candidate>,
}

impl ExtensionInstallLocal {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionInstallLocal {
    const ID: &'static str = "extension.install_local";
    const TITLE: &'static str = "Install an unsigned extension from a local build";
    const RISK: Risk = Risk::Privileged;
    const PERSON_ONLY: bool = true;

    type Params = LocalParams;
    type Output = ExtensionInfo;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &LocalParams,
    ) -> Result<(), ActionError> {
        if PathBuf::from(&params.path).is_absolute() {
            return Ok(());
        }

        Err(ActionError::InvalidParams(
            "the path must be absolute".to_owned(),
        ))
    }

    fn inspect(
        &self,
        _target: Option<&ResourceRef>,
        params: &mut LocalParams,
    ) -> Result<Option<Value>, ActionError> {
        let candidate = self
            .extensions
            .examine_local(&PathBuf::from(&params.path))?;
        let shown = candidate.shown(&params.path);

        params.candidate = Some(candidate);
        Ok(Some(shown))
    }

    fn recorded_params(&self, params: &Value) -> Value {
        let mut recorded = params.clone();

        if let Some(fields) = recorded.as_object_mut() {
            fields.insert("signed".to_owned(), Value::Bool(false));
        }

        recorded
    }

    fn summary(&self, output: &ExtensionInfo) -> Value {
        output.install_summary()
    }

    async fn run(
        &self,
        _context: &Context,
        params: LocalParams,
    ) -> Result<ExtensionInfo, ActionError> {
        let candidate = params
            .candidate
            .ok_or_else(|| ActionError::failed("the build was not checked before install"))?;
        let extensions = self.extensions.clone();

        blocking(move || extensions.install(candidate)).await
    }
}
