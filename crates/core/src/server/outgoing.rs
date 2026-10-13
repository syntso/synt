use bytes::Bytes;
use futures_util::SinkExt;
use synt_platform::LocalStream;
use synt_proto::{Frame, FrameWriter, Message, RequestId, Response, RpcError};
use tokio::sync::mpsc;

pub(super) type Outgoing = FrameWriter<LocalStream>;

pub(super) async fn write_all(mut outgoing: Outgoing, mut queue: mpsc::Receiver<Message>) {
    let limit = outgoing.encoder().limit();

    while let Some(first) = queue.recv().await {
        let mut next = Some(first);

        while let Some(message) = next {
            if let Some(body) = encode(&message, limit)
                && outgoing.feed(Frame::message(body)).await.is_err()
            {
                return;
            }

            next = queue.try_recv().ok();
        }

        if outgoing.flush().await.is_err() {
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
