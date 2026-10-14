use schemars::JsonSchema;
use serde::Deserialize;
use synt_actions::ActionError;

use crate::passwords::PasswordName;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PasswordNameParams {
    pub name: String,
}

impl PasswordNameParams {
    pub(crate) fn password_name(&self) -> Result<PasswordName, ActionError> {
        PasswordName::parse(&self.name).map_err(ActionError::InvalidParams)
    }
}
