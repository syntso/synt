use serde::Serialize;
use serde_json::{Value, json};

use crate::{ExtensionError, Extensions};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct SettingsValues {
    pub extension: String,
    pub schema: Value,
    pub values: Value,
}

impl SettingsValues {
    pub(crate) fn current(extensions: &Extensions, id: &str) -> Result<Self, ExtensionError> {
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

        Ok(Self {
            extension: id.to_owned(),
            schema,
            values,
        })
    }
}
