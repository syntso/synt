use synt_proto::{Frame, Message, Notice, RunbookStep};
use tokio::sync::mpsc;

use super::Progress;

pub(crate) struct LinkProgress {
    frames: mpsc::Sender<Frame>,
}

impl LinkProgress {
    pub(crate) fn new(frames: mpsc::Sender<Frame>) -> Self {
        Self { frames }
    }
}

impl Progress for LinkProgress {
    async fn report(&self, step: RunbookStep) {
        let Ok(notification) = step.to_notification() else {
            return;
        };
        let Ok(body) = Message::from(notification).encode() else {
            return;
        };

        let _ = self.frames.send(Frame::message(body)).await;
    }
}
