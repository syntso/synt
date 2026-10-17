use std::sync::Arc;

use synt_proto::StreamId;
use tokio::sync::mpsc;

use super::{StreamEvent, Streams};

pub struct StreamReceiver {
    stream: StreamId,
    events: mpsc::Receiver<StreamEvent>,
    streams: Arc<Streams>,
}

impl StreamReceiver {
    pub(crate) fn new(
        stream: StreamId,
        events: mpsc::Receiver<StreamEvent>,
        streams: Arc<Streams>,
    ) -> Self {
        Self {
            stream,
            events,
            streams,
        }
    }

    pub fn stream(&self) -> StreamId {
        self.stream
    }

    pub async fn next(&mut self) -> Option<StreamEvent> {
        self.events.recv().await
    }
}

impl Drop for StreamReceiver {
    fn drop(&mut self) {
        self.streams.release(self.stream);
    }
}
