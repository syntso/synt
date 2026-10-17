use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

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
    extension: String,
    values: Value,
}

impl ExtensionSettingsSet {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }

    fn save(&self, params: &SetParams) -> Result<SettingsValues, ExtensionError> {
        let current = SettingsValues::current(&self.extensions, &params.extension)?;

        current.schema().check(&params.values)?;

        self.extensions
            .services()
            .shelf()
            .settings()
            .put(&params.extension, &params.values)?;

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
        let keys: Vec<&String> = params
            .get("values")
            .and_then(Value::as_object)
            .map(|values| values.keys().collect())
            .unwrap_or_default();

        json!({ "extension": params.get("extension"), "keys": keys })
    }

    fn summary(&self, output: &SettingsValues) -> Value {
        json!({ "extension": output.extension(), "keys": output.keys() })
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
