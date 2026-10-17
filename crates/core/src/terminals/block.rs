mod collector;
mod format_character;
mod history;
mod mark;
mod open;
mod percent;
mod scanner;
mod sighting;

pub(crate) use history::BlockHistory;
pub(crate) use scanner::Scanner;

use collector::Collector;
use format_character::is_format_character;
use mark::Mark;
use open::OpenBlock;
use percent::percent_decode;
use sighting::Sighting;

use serde::Serialize;
use synt_proto::{Millis, SessionBlock, StreamId};

use super::SessionId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Block {
    #[serde(rename = "block")]
    pub number: u64,
    pub command: Option<String>,
    pub exit: Option<i32>,
    pub duration_ms: Millis,
    pub output_start: u64,
    pub output_end: u64,
}

impl Block {
    pub(crate) fn notice(&self, session: SessionId, stream: StreamId) -> SessionBlock {
        SessionBlock {
            session,
            stream,
            block: self.number,
            command: self.command.clone(),
            exit: self.exit,
            duration_ms: self.duration_ms,
            output_start: self.output_start,
            output_end: self.output_end,
        }
    }
}
