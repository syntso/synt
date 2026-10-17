use serde::Serialize;

use crate::ObjectRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Deleted {
    #[serde(flatten)]
    pub object: ObjectRef,
    pub gone: bool,
}
