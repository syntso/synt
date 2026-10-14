use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::get::current;
use super::set_params::SetParams;
use super::settings_values::SettingsValues;
use crate::builtin::blocking::blocking;
use crate::settings::SettingsCheck;
use crate::{ExtensionError, Extensions};

#[derive(Clone)]
pub(crate) struct ExtensionSettingsSet {
    extensions: Extensions,
}

impl ExtensionSettingsSet {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionSettingsSet {
    const ID: &'static str = "extension.settings.set";
    const TITLE: &'static str = "Save the settings of an extension";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = SetParams;
    type Output = SettingsValues;

    fn recorded_params(&self, params: &Value) -> Value {
        json!({
            "extension": params.get("extension"),
            "keys": keys(params.get("values")),
        })
    }

    fn summary(&self, output: &SettingsValues) -> Value {
        json!({ "extension": output.extension, "keys": keys(Some(&output.values)) })
    }

    async fn run(
        &self,
        _context: &Context,
        params: SetParams,
    ) -> Result<SettingsValues, ActionError> {
        let extensions = self.extensions.clone();

        blocking(move || {
            let SettingsValues { schema, .. } = current(&extensions, &params.extension)?;
            SettingsCheck::new(&schema)
                .check(&params.values)
                .map_err(ExtensionError::Refused)?;

            extensions
                .services()
                .shelf
                .put_settings(&params.extension, &params.values)?;

            current(&extensions, &params.extension)
        })
        .await
    }
}

fn keys(values: Option<&Value>) -> Vec<&String> {
    values
        .and_then(Value::as_object)
        .map(|values| values.keys().collect())
        .unwrap_or_default()
}
