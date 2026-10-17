use serde::Serialize;
use synt_actions::Context;
use synt_proto::StreamId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct StreamStarted {
    pub(crate) request: u64,
    pub(crate) stream: u32,
}

impl StreamStarted {
    pub(crate) fn new(context: &Context, stream: StreamId) -> Self {
        Self {
            request: context.request().get(),
            stream: stream.0,
        }
    }
}
