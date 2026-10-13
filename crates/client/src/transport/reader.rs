use std::sync::Arc;

use futures_util::StreamExt;
use synt_proto::{Frame, FrameCodec, Message, Notification};
use tokio::io::AsyncRead;
use tokio::sync::broadcast;
use tokio_util::codec::FramedRead;

use super::pending::Pending;

pub(super) async fn route<R>(
    mut incoming: FramedRead<R, FrameCodec>,
    pending: Arc<Pending>,
    notifications: broadcast::Sender<Notification>,
) where
    R: AsyncRead + Unpin,
{
    while let Some(Ok(frame)) = incoming.next().await {
        match message_in(frame) {
            Some(Message::Response(response)) => pending.resolve(response),
            Some(Message::Notification(notification)) => {
                let _ = notifications.send(notification);
            }
            Some(Message::Request(_)) | None => {}
        }
    }

    pending.close();
}

fn message_in(frame: Frame) -> Option<Message> {
    let Frame::Message(body) = frame else {
        return None;
    };

    Message::decode(&body).ok()
}
