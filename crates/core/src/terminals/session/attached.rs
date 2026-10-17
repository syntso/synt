use serde::Serialize;
use synt_proto::StreamId;

use super::SessionId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AttachedSession {
    pub session: SessionId,
    pub stream: StreamId,
    #[serde(rename = "cols")]
    pub columns: u16,
    pub rows: u16,
    pub replayed_bytes: u64,
}
