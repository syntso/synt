use serde::Serialize;

use crate::ObjectRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Scaled {
    #[serde(flatten)]
    pub object: ObjectRef,
    pub from: Option<i32>,
    pub to: i32,
}
