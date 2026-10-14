use serde::Serialize;

use super::EndReason;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct StreamEnd {
    pub(crate) reason: EndReason,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,

    pub(crate) bytes: u64,
    pub(crate) items: u64,
    pub(crate) duration_ms: u64,
}
