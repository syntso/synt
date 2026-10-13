use bytes::Bytes;
use futures_util::SinkExt;
use synt_platform::LocalStream;
use synt_proto::{Frame, FrameWriter, Message, RequestId, Response, RpcError};
use tokio::sync::mpsc;

pub(super) type Outgoing = FrameWriter<LocalStream>;

pub(super) async fn write_all(mut outgoing: Outgoing, mut queue: mpsc::Receiver<Response>) {
    let limit = outgoing.encoder().limit();

    while let Some(first) = queue.recv().await {
        let mut next = Some(first);

        while let Some(response) = next {
            let frame = Frame::message(encode(response, limit));

            if outgoing.feed(frame).await.is_err() {
                return;
            }

            next = queue.try_recv().ok();
        }

        if outgoing.flush().await.is_err() {
            return;
        }
    }
}

fn encode(response: Response, limit: usize) -> Bytes {
    let id = response.id;

    match Message::from(response).encode() {
        Ok(body) if body.len() <= limit => body,
        Ok(_) => refusal(id, "the reply exceeds the frame limit"),
        Err(error) => {
            eprintln!("synt-core: reply: {error}");
            refusal(id, "the reply cannot be encoded")
        }
    }
}

fn refusal(id: Option<RequestId>, reason: &str) -> Bytes {
    let response = Response::new(id, Err(RpcError::internal(reason)));

    Message::from(response)
        .encode()
        .expect("a response holding only an id and a message always encodes")
}
