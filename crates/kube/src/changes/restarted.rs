use serde::Serialize;

use crate::ObjectRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Restarted {
    #[serde(flatten)]
    pub object: ObjectRef,
    pub restarted_at: String,
}
