use serde::Serialize;
use synt_actions::Risk;
use synt_proto::Requester;

use crate::terminal::{SessionId, SessionKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionInfo {
    pub session: SessionId,
    pub kind: SessionKind,
    pub target: Option<String>,
    pub host: String,
    pub actor: Requester,
    pub risk: Risk,
    #[serde(rename = "cols")]
    pub columns: u16,
    pub rows: u16,
    pub attached: usize,
    pub blocks: usize,
    pub age_ms: u64,
}
