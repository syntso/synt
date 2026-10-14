use schemars::JsonSchema;
use serde::Deserialize;
use synt_actions::ActionError;

use super::names::{check_kind, check_namespace, check_object_name};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct ObjectParams {
    pub kind: String,
    pub name: String,

    #[serde(default)]
    pub namespace: Option<String>,
}

impl ObjectParams {
    pub(crate) fn check(&self) -> Result<(), ActionError> {
        check_kind(&self.kind)?;
        check_object_name("name", &self.name)?;
        check_namespace(self.namespace.as_deref())
    }
}
