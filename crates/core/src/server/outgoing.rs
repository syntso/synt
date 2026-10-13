use bytes::Bytes;
use futures_util::SinkExt;
use synt_platform::LocalStream;
use synt_proto::{Frame, FrameWriter, Message, RequestId, Response, RpcError};
use tokio::sync::mpsc;

pub(super) type Outgoing = FrameWriter<LocalStream>;

pub(super) async fn write_all(
    mut outgoing: Outgoing,
    mut queue: mpsc::Receiver<Message>,
    mut frames: mpsc::Receiver<Frame>,
) {
    let limit = outgoing.encoder().limit();

    loop {
        let next = tokio::select! {
            message = queue.recv() => {
                let Some(message) = message else {
                    return;
                };
                encode(&message, limit).map(Frame::message)
            }
            Some(frame) = frames.recv() => Some(frame),
        };

        let Some(frame) = next else {
            continue;
        };

        if outgoing.feed(frame).await.is_err() {
            return;
        }

        let idle = queue.is_empty() && frames.is_empty();

        if idle && outgoing.flush().await.is_err() {
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
            eprintln!("synt-core: reply: {error}");
            "the reply cannot be encoded"
        }
    };

    reply_to.map(|id| refusal(id, problem))
}

fn refusal(id: Option<RequestId>, reason: &str) -> Bytes {
    let response = Response::new(id, Err(RpcError::internal(reason)));

    Message::from(response)
        .encode()
        .expect("a response holding only an id and a message always encodes")
}
