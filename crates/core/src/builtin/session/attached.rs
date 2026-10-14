use serde::Serialize;
use synt_proto::StreamId;

use crate::terminal::{Attached, SessionId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AttachedSession {
    pub session: SessionId,
    pub stream: StreamId,
    #[serde(rename = "cols")]
    pub columns: u16,
    pub rows: u16,
    pub replayed_bytes: u64,
}

impl AttachedSession {
    pub(crate) fn new(session: SessionId, attached: &Attached) -> Self {
        Self {
            session,
            stream: attached.stream,
            columns: attached.size.columns,
            rows: attached.size.rows,
            replayed_bytes: attached.replayed,
        }
    }
}
