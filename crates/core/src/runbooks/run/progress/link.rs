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

    fn frame(step: &RunbookStep) -> Result<Frame, serde_json::Error> {
        let notification = step.to_notification()?;
        let body = Message::from(notification).encode()?;

        Ok(Frame::message(body))
    }
}

impl Progress for LinkProgress {
    async fn report(&self, step: RunbookStep) {
        let frame = match Self::frame(&step) {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!(
                    "synt-core: step {} of run {} cannot be encoded: {error}",
                    step.step, step.run
                );
                return;
            }
        };

        if self.frames.send(frame).await.is_err() {
            eprintln!(
                "synt-core: run {} goes on unseen, its connection is closed",
                step.run
            );
        }
    }
}
