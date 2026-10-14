use serde::{Deserialize, Serialize};

use crate::encoding::Base64;
use crate::id::MemberId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) body: Option<Base64>,
    pub(crate) conflicts: Vec<String>,
    pub(crate) author: Option<MemberId>,
}
