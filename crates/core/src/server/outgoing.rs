use bytes::Bytes;
use futures_util::SinkExt;
use synt_platform::LocalStream;
use synt_proto::{Frame, FrameWriter, Message, RequestId, Response, RpcError};
use tokio::sync::mpsc;

use crate::log::log;

pub(super) struct Outgoing {
    writer: FrameWriter<LocalStream>,
}

impl Outgoing {
    pub(super) fn new(writer: FrameWriter<LocalStream>) -> Self {
        Self { writer }
    }

    pub(super) async fn write_all(
        mut self,
        mut queue: mpsc::Receiver<Message>,
        mut frames: mpsc::Receiver<Frame>,
    ) {
        let limit = self.writer.encoder().limit();

        loop {
            let next = tokio::select! {
                message = queue.recv() => {
                    let Some(message) = message else {
                        return;
                    };
                    Self::encode(&message, limit).map(Frame::message)
                }
                Some(frame) = frames.recv() => Some(frame),
            };

            let Some(frame) = next else {
                continue;
            };

            if self.writer.feed(frame).await.is_err() {
                return;
            }

            let idle = queue.is_empty() && frames.is_empty();

            if idle && self.writer.flush().await.is_err() {
                return;
            }
        }
    }

    fn encode(message: &Message, limit: usize) -> Option<Bytes> {
        let reply_to = match message {
            Message::Response(response) => Some(response.id),
            Message::Request(_) | Message::Notification(_) => None,
        };

        let problem = match message.encode() {
            Ok(body) if body.len() <= limit => return Some(body),
            Ok(_) => "the reply exceeds the frame limit",
            Err(error) => {
                log!("reply: {error}");
                "the reply cannot be encoded"
            }
        };

        reply_to.and_then(|id| Self::refusal(id, problem))
    }

    fn refusal(id: Option<RequestId>, reason: &str) -> Option<Bytes> {
        let response = Response::new(id, Err(RpcError::internal(reason)));

        Message::from(response).encode().ok()
    }
}
