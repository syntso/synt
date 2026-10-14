use serde::Serialize;

use super::Change;
use crate::ObjectRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppliedObject {
    #[serde(flatten)]
    pub object: ObjectRef,
    pub change: Change,
}
