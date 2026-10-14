use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::values::SettingsValues;
use crate::Extensions;
use crate::builtin::blocking::blocking;
use crate::builtin::params::ExtensionParams;

#[derive(Clone)]
pub(crate) struct ExtensionSettingsGet {
    extensions: Extensions,
}

impl ExtensionSettingsGet {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionSettingsGet {
    const ID: &'static str = "extension.settings.get";
    const TITLE: &'static str = "Read the settings form and values of an extension";
    const RISK: Risk = Risk::Read;

    type Params = ExtensionParams;
    type Output = SettingsValues;

    fn summary(&self, output: &SettingsValues) -> Value {
        json!({ "extension": output.extension })
    }

    async fn run(
        &self,
        _context: &Context,
        params: ExtensionParams,
    ) -> Result<SettingsValues, ActionError> {
        let extensions = self.extensions.clone();

        blocking(move || SettingsValues::current(&extensions, &params.extension)).await
    }
}
