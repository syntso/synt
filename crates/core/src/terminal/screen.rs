use std::time::Instant;

use bytes::Bytes;
use synt_proto::{Frame, Message, Notification, StreamId};
use synt_ssh::TerminalSize;

use super::{Attachment, Block, Blocks, Ending, Replay, Scanner, SessionId};

const REPLAY_LIMIT: usize = 1024 * 1024;
const REPLAY_CHUNK: usize = 32 * 1024;

pub(super) struct Screen {
    session: SessionId,
    replay: Replay,
    scanner: Scanner,
    blocks: Blocks,
    attachments: Vec<Attachment>,
    size: TerminalSize,
    ending: Option<Ending>,
}

impl Screen {
    pub(super) fn new(session: SessionId, size: TerminalSize) -> Self {
        Self {
            session,
            replay: Replay::new(REPLAY_LIMIT),
            scanner: Scanner::default(),
            blocks: Blocks::default(),
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

    pub(super) fn blocks(&self) -> Vec<Block> {
        self.blocks.list()
    }

    pub(super) fn block_count(&self) -> u64 {
        self.blocks.count()
    }

    pub(super) async fn show(&mut self, data: Bytes) {
        let finished = self.read_marks(&data);
        self.replay.push(&data);

        self.send_all(|stream| {
            Some(Frame::Stream {
                stream,
                data: data.clone(),
            })
        })
        .await;

        for block in finished {
            let session = self.session;
            self.send_all(|stream| {
                message_frame(block.notice(session, stream).to_notification().ok()?)
            })
            .await;
        }
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

    fn read_marks(&mut self, data: &[u8]) -> Vec<Block> {
        let sightings = self.scanner.scan(self.replay.total(), data);
        let now = Instant::now();

        sightings
            .into_iter()
            .filter_map(|sighting| self.blocks.see(sighting, now))
            .collect()
    }

    async fn send_all(&mut self, frame_for: impl Fn(StreamId) -> Option<Frame>) {
        let mut kept = Vec::with_capacity(self.attachments.len());

        for attachment in self.attachments.drain(..) {
            let sent = match frame_for(attachment.stream()) {
                Some(frame) => attachment.send(frame).await,
                None => true,
            };

            if sent {
                kept.push(attachment);
            }
        }

        self.attachments = kept;
    }
}

async fn announce(attachment: &Attachment, ending: &Ending) {
    let Some(frame) = ending
        .notice(attachment.stream())
        .to_notification()
        .ok()
        .and_then(message_frame)
    else {
        return;
    };

    attachment.send(frame).await;
}

fn message_frame(notification: Notification) -> Option<Frame> {
    let body = Message::from(notification).encode().ok()?;

    Some(Frame::message(body))
}
