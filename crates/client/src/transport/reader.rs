use std::sync::Arc;

use futures_util::StreamExt;
use synt_proto::{Frame, FrameCodec, Message, Response};
use tokio::io::AsyncRead;
use tokio_util::codec::FramedRead;

use super::pending::Pending;

pub(super) async fn route<R>(mut incoming: FramedRead<R, FrameCodec>, pending: Arc<Pending>)
where
    R: AsyncRead + Unpin,
{
    while let Some(Ok(frame)) = incoming.next().await {
        if let Some(response) = response_in(frame) {
            pending.resolve(response);
        }
    }

    pending.close();
}

fn response_in(frame: Frame) -> Option<Response> {
    let Frame::Message(body) = frame else {
        return None;
    };

    match Message::decode(&body) {
        Ok(Message::Response(response)) => Some(response),
        Ok(Message::Request(_) | Message::Notification(_)) | Err(_) => None,
    }
}
