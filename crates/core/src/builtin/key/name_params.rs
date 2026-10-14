use schemars::JsonSchema;
use serde::Deserialize;
use synt_actions::ActionError;

use crate::ssh_keys::KeyName;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KeyNameParams {
    pub name: String,
}

impl KeyNameParams {
    pub(crate) fn key_name(&self) -> Result<KeyName, ActionError> {
        KeyName::parse(&self.name).map_err(ActionError::InvalidParams)
    }
}
