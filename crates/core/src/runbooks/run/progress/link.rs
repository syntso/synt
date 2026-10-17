use synt_proto::{Frame, Message, Notice, RunbookStep};
use tokio::sync::mpsc;

use super::Progress;

use crate::log::log;

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
                log!(
                    "step {} of run {} cannot be encoded: {error}",
                    step.step,
                    step.run
                );
                return;
            }
        };

        if self.frames.send(frame).await.is_err() {
            log!("run {} goes on unseen, its connection is closed", step.run);
        }
    }
}
