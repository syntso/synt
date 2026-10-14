use serde::Serialize;
use synt_proto::{EndReason, SessionEnded, StreamId};

use super::SessionId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ending {
    pub session: SessionId,
    pub reason: EndReason,
    pub exit: Option<u32>,
    pub signal: Option<String>,
    pub duration_ms: u64,
    pub output_bytes: u64,
    pub blocks: u64,
    pub kex: Option<String>,
    pub cipher: Option<String>,
}

impl Ending {
    pub(crate) fn notice(&self, stream: StreamId) -> SessionEnded {
        SessionEnded {
            session: self.session.get(),
            stream,
            reason: self.reason,
            exit: self.exit,
            signal: self.signal.clone(),
            duration_ms: self.duration_ms,
        }
    }
}
