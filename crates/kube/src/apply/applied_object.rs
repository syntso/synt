use serde::Serialize;

use crate::{Change, ObjectRef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppliedObject {
    #[serde(flatten)]
    pub object: ObjectRef,
    pub change: Change,
}
