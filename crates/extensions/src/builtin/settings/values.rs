use serde::Serialize;
use serde_json::{Value, json};

use crate::manifest::SettingsSchema;
use crate::{ExtensionError, Extensions};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct SettingsValues {
    extension: String,
    schema: SettingsSchema,
    values: Value,
}

impl SettingsValues {
    pub(crate) fn current(extensions: &Extensions, id: &str) -> Result<Self, ExtensionError> {
        let loaded = extensions.loaded(id)?;
        let schema = loaded
            .guest()
            .manifest()
            .contributes()
            .settings()
            .map(|settings| settings.schema().clone())
            .ok_or_else(|| ExtensionError::NoSettings(id.to_owned()))?;

        let stored = extensions.services().shelf().settings().get(id)?;

        Ok(Self {
            extension: id.to_owned(),
            schema,
            values: stored.unwrap_or_else(|| json!({})),
        })
    }

    pub(crate) fn schema(&self) -> &SettingsSchema {
        &self.schema
    }

    pub(crate) fn summary(&self) -> Value {
        json!({ "extension": self.extension })
    }

    pub(crate) fn keys(&self) -> Vec<&String> {
        self.values
            .as_object()
            .map(|values| values.keys().collect())
            .unwrap_or_default()
    }

    pub(crate) fn extension(&self) -> &str {
        &self.extension
    }
}
