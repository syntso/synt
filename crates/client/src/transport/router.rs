use std::sync::Arc;

use futures_util::StreamExt;
use synt_proto::{Frame, FrameCodec, Message, Notification, SessionEnded};
use tokio::io::AsyncRead;
use tokio::sync::broadcast;
use tokio_util::codec::FramedRead;

use super::pending::Pending;
use crate::StreamEvent;
use crate::streams::Streams;

pub(super) struct Router<R> {
    incoming: FramedRead<R, FrameCodec>,
    pending: Arc<Pending>,
    notifications: broadcast::Sender<Notification>,
    streams: Arc<Streams>,
}

impl<R: AsyncRead + Unpin> Router<R> {
    pub(super) fn new(
        incoming: FramedRead<R, FrameCodec>,
        pending: Arc<Pending>,
        notifications: broadcast::Sender<Notification>,
        streams: Arc<Streams>,
    ) -> Self {
        Self {
            incoming,
            pending,
            notifications,
            streams,
        }
    }

    pub(super) async fn run(mut self) {
        while let Some(Ok(frame)) = self.incoming.next().await {
            match frame {
                Frame::Message(body) => self.message(&body),
                Frame::Stream { stream, data } => {
                    self.streams.deliver(stream, StreamEvent::Data(data));
                }
            }
        }

        self.pending.close();
        self.streams.close();
    }

    fn message(&self, body: &[u8]) {
        match Message::decode(body).ok() {
            Some(Message::Response(response)) => self.pending.resolve(response),
            Some(Message::Notification(notification)) => self.notify(notification),
            Some(Message::Request(_)) | None => {}
        }
    }

    fn notify(&self, notification: Notification) {
        if let Some(Ok(ended)) = notification.read::<SessionEnded>() {
            let stream = ended.stream;

            self.streams.deliver(stream, StreamEvent::Ended(ended));
            self.streams.end(stream);
        }

        let _ = self.notifications.send(notification);
    }
}
