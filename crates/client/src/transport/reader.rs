use std::sync::Arc;

use futures_util::StreamExt;
use synt_proto::{Frame, FrameCodec, Message, Notification, SessionEnded};
use tokio::io::AsyncRead;
use tokio::sync::broadcast;
use tokio_util::codec::FramedRead;

use super::Streams;
use super::pending::Pending;
use crate::StreamEvent;

pub(super) async fn route<R>(
    mut incoming: FramedRead<R, FrameCodec>,
    pending: Arc<Pending>,
    notifications: broadcast::Sender<Notification>,
    streams: Arc<Streams>,
) where
    R: AsyncRead + Unpin,
{
    while let Some(Ok(frame)) = incoming.next().await {
        let body = match frame {
            Frame::Message(body) => body,
            Frame::Stream { stream, data } => {
                streams.deliver(stream, StreamEvent::Data(data));
                continue;
            }
        };

        match Message::decode(&body).ok() {
            Some(Message::Response(response)) => pending.resolve(response),
            Some(Message::Notification(notification)) => {
                end_stream(&streams, &notification);
                let _ = notifications.send(notification);
            }
            Some(Message::Request(_)) | None => {}
        }
    }

    pending.close();
    streams.close();
}

fn end_stream(streams: &Streams, notification: &Notification) {
    let Some(Ok(ended)) = notification.read::<SessionEnded>() else {
        return;
    };

    let stream = ended.stream;
    streams.deliver(stream, StreamEvent::Ended(ended));
    streams.end(stream);
}
