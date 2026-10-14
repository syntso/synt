use schemars::JsonSchema;
use serde::Deserialize;
use synt_actions::ActionError;

use crate::secrets::SecretName;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SecretNameParams {
    pub name: String,
}

impl SecretNameParams {
    pub(crate) fn secret_name(&self) -> Result<SecretName, ActionError> {
        SecretName::parse(&self.name).map_err(ActionError::InvalidParams)
    }
}
