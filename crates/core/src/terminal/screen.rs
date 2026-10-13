use bytes::Bytes;
use synt_proto::{Frame, Message, StreamId};
use synt_ssh::TerminalSize;

use super::{Attachment, Ending, Replay};

const REPLAY_LIMIT: usize = 1024 * 1024;
const REPLAY_CHUNK: usize = 32 * 1024;

pub(super) struct Screen {
    replay: Replay,
    attachments: Vec<Attachment>,
    size: TerminalSize,
    ending: Option<Ending>,
}

impl Screen {
    pub(super) fn new(size: TerminalSize) -> Self {
        Self {
            replay: Replay::new(REPLAY_LIMIT),
            attachments: Vec::new(),
            size,
            ending: None,
        }
    }

    pub(super) fn size(&self) -> TerminalSize {
        self.size
    }

    pub(super) fn resize(&mut self, size: TerminalSize) {
        self.size = size;
    }

    pub(super) fn attached(&mut self) -> usize {
        self.attachments.retain(|attachment| !attachment.is_gone());
        self.attachments.len()
    }

    pub(super) fn output_bytes(&self) -> u64 {
        self.replay.total()
    }

    pub(super) async fn show(&mut self, data: Bytes) {
        self.replay.push(&data);

        let mut kept = Vec::with_capacity(self.attachments.len());

        for attachment in self.attachments.drain(..) {
            let frame = Frame::Stream {
                stream: attachment.stream(),
                data: data.clone(),
            };

            if attachment.send(frame).await {
                kept.push(attachment);
            }
        }

        self.attachments = kept;
    }

    pub(super) async fn attach(&mut self, attachment: Attachment) -> u64 {
        let snapshot = Bytes::from(self.replay.snapshot());
        let replayed = u64::try_from(snapshot.len()).unwrap_or(u64::MAX);

        for start in (0..snapshot.len()).step_by(REPLAY_CHUNK) {
            let end = (start + REPLAY_CHUNK).min(snapshot.len());
            let frame = Frame::Stream {
                stream: attachment.stream(),
                data: snapshot.slice(start..end),
            };

            if !attachment.send(frame).await {
                return replayed;
            }
        }

        match &self.ending {
            Some(ending) => announce(&attachment, ending).await,
            None => self.attachments.push(attachment),
        }

        replayed
    }

    pub(super) async fn end(&mut self, ending: Ending) {
        for attachment in self.attachments.drain(..) {
            announce(&attachment, &ending).await;
        }

        self.ending = Some(ending);
    }
}

async fn announce(attachment: &Attachment, ending: &Ending) {
    let Some(frame) = ended_frame(attachment.stream(), ending) else {
        return;
    };

    attachment.send(frame).await;
}

fn ended_frame(stream: StreamId, ending: &Ending) -> Option<Frame> {
    let notification = ending.notice(stream).to_notification().ok()?;
    let body = Message::from(notification).encode().ok()?;

    Some(Frame::message(body))
}
