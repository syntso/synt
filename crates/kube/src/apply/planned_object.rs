use serde::Serialize;

use super::Change;
use crate::ObjectRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlannedObject {
    #[serde(flatten)]
    pub object: ObjectRef,
    pub change: Change,
    pub server_checked: bool,

    #[serde(skip)]
    pub diff: String,
}
