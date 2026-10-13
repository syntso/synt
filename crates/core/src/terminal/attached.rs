use synt_proto::StreamId;
use synt_ssh::TerminalSize;

pub(crate) struct Attached {
    pub(crate) stream: StreamId,
    pub(crate) size: TerminalSize,
    pub(crate) replayed: u64,
}
