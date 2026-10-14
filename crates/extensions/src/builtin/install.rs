use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::ExtensionInfo;
use super::blocking::{blocking, failure};
use crate::Extensions;
use crate::extension::Candidate;

#[derive(Clone)]
pub struct ExtensionInstall {
    extensions: Extensions,
}

#[derive(Deserialize, JsonSchema)]
pub struct InstallParams {
    pub id: String,

    #[serde(skip)]
    #[schemars(skip)]
    candidate: Option<Candidate>,
}

impl ExtensionInstall {
    pub fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionInstall {
    const ID: &'static str = "extension.install";
    const TITLE: &'static str = "Install a signed extension from the registry";
    const RISK: Risk = Risk::Privileged;
    const PERSON_ONLY: bool = true;

    type Params = InstallParams;
    type Output = ExtensionInfo;

    fn inspect(
        &self,
        _target: Option<&ResourceRef>,
        params: &mut InstallParams,
    ) -> Result<Option<Value>, ActionError> {
        let candidate = self
            .extensions
            .examine_listed(&params.id)
            .map_err(|error| failure(&error))?;
        let shown = candidate.shown("registry");

        params.candidate = Some(candidate);
        Ok(Some(shown))
    }

    fn summary(&self, output: &ExtensionInfo) -> Value {
        output.install_summary()
    }

    async fn run(
        &self,
        _context: &Context,
        params: InstallParams,
    ) -> Result<ExtensionInfo, ActionError> {
        let candidate = params
            .candidate
            .ok_or_else(|| ActionError::failed("the extension was not checked before install"))?;
        let extensions = self.extensions.clone();

        blocking(move || extensions.install(candidate)).await
    }
}
