use std::sync::{Arc, Weak};
use std::time::Duration;

use synt_proto::{Frame, StreamId};
use tokio::sync::mpsc;
use tokio::time;

use crate::link::Link;

const SEND_LIMIT: Duration = Duration::from_secs(10);

pub(super) struct Attachment {
    stream: StreamId,
    frames: mpsc::Sender<Frame>,
    link: Weak<Link>,
}

impl Attachment {
    pub(super) fn new(stream: StreamId, link: &Arc<Link>) -> Self {
        Self {
            stream,
            frames: link.frames(),
            link: Arc::downgrade(link),
        }
    }

    pub(super) fn stream(&self) -> StreamId {
        self.stream
    }

    pub(super) fn is_gone(&self) -> bool {
        self.frames.is_closed()
    }

    pub(super) async fn send(&self, frame: Frame) -> bool {
        let sent = time::timeout(SEND_LIMIT, self.frames.send(frame)).await;

        matches!(sent, Ok(Ok(())))
    }
}

impl Drop for Attachment {
    fn drop(&mut self) {
        if let Some(link) = self.link.upgrade() {
            link.forget(self.stream);
        }
    }
}
