use serde::Serialize;
use synt_proto::{SessionBlock, StreamId};

use crate::terminal::SessionId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Block {
    pub block: u64,
    pub command: Option<String>,
    pub exit: Option<i32>,
    pub duration_ms: u64,
    pub output_start: u64,
    pub output_end: u64,
}

impl Block {
    pub(crate) fn notice(&self, session: SessionId, stream: StreamId) -> SessionBlock {
        SessionBlock {
            session: session.value(),
            stream,
            block: self.block,
            command: self.command.clone(),
            exit: self.exit,
            duration_ms: self.duration_ms,
            output_start: self.output_start,
            output_end: self.output_end,
        }
    }
}
