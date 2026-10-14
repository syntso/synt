use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::settings_values::SettingsValues;
use crate::builtin::blocking::blocking;
use crate::builtin::extension_params::ExtensionParams;
use crate::{ExtensionError, Extensions};

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

        blocking(move || current(&extensions, &params.extension)).await
    }
}

pub(super) fn current(extensions: &Extensions, id: &str) -> Result<SettingsValues, ExtensionError> {
    let loaded = extensions.loaded(id)?;
    let schema = loaded
        .guest
        .manifest()
        .contributes
        .settings
        .as_ref()
        .map(|settings| settings.schema.clone())
        .ok_or_else(|| ExtensionError::Refused(format!("{id} contributes no settings")))?;

    let values = extensions
        .services()
        .shelf
        .settings(id)?
        .unwrap_or_else(|| json!({}));

    Ok(SettingsValues {
        extension: id.to_owned(),
        schema,
        values,
    })
}
