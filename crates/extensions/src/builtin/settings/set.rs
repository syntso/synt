use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::check::SettingsCheck;
use super::values::SettingsValues;
use crate::blocking::Blocking;
use crate::{ExtensionError, Extensions};

#[derive(Clone)]
pub(crate) struct ExtensionSettingsSet {
    extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetParams {
    pub extension: String,
    pub values: Value,
}

impl ExtensionSettingsSet {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }

    fn save(&self, params: &SetParams) -> Result<SettingsValues, ExtensionError> {
        let SettingsValues { schema, .. } =
            SettingsValues::current(&self.extensions, &params.extension)?;

        SettingsCheck::new(&schema)
            .check(&params.values)
            .map_err(ExtensionError::Refused)?;

        self.extensions
            .services()
            .shelf
            .put_settings(&params.extension, &params.values)?;

        SettingsValues::current(&self.extensions, &params.extension)
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
        let action = self.clone();

        Blocking::run(move || action.save(&params)).await
    }
}

fn keys(values: Option<&Value>) -> Vec<&String> {
    values
        .and_then(Value::as_object)
        .map(|values| values.keys().collect())
        .unwrap_or_default()
}
