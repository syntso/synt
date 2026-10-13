use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use bytes::Bytes;
use futures_util::SinkExt;
use serde::Serialize;
use serde_json::Value;
use synt_proto::{
    Frame, FrameCodec, FrameError, FrameWriter, Message, Method, Notification, Request, RequestId,
    StreamId, split_framed,
};
use tokio::sync::{Mutex, broadcast};
use tokio::task::JoinHandle;

use super::Duplex;
use super::Streams;
use super::pending::Pending;
use super::reader;
use crate::{ClientError, StreamReceiver};

type Outgoing = FrameWriter<Box<dyn Duplex>>;

const NOTIFICATION_BACKLOG: usize = 64;
const STREAM_CHUNK: usize = 32 * 1024;

pub(crate) struct Connection {
    outgoing: Mutex<Outgoing>,
    pending: Arc<Pending>,
    notifications: broadcast::WeakSender<Notification>,
    streams: Arc<Streams>,
    next_id: AtomicU64,
    next_stream: AtomicU32,
    reader: JoinHandle<()>,
}

impl Connection {
    pub(crate) fn open(stream: impl Duplex + 'static) -> Self {
        let stream: Box<dyn Duplex> = Box::new(stream);
        let (incoming, outgoing) = split_framed(stream, FrameCodec::DEFAULT_LIMIT);

        let pending = Arc::new(Pending::new());
        let (notifications, _) = broadcast::channel(NOTIFICATION_BACKLOG);

        let streams = Arc::new(Streams::new());

        let routed = reader::route(
            incoming,
            Arc::clone(&pending),
            notifications.clone(),
            Arc::clone(&streams),
        );
        let notifications = notifications.downgrade();
        let reader = tokio::spawn(routed);

        Self {
            outgoing: Mutex::new(outgoing),
            pending,
            notifications,
            streams,
            next_id: AtomicU64::new(1),
            next_stream: AtomicU32::new(StreamId::FIRST_OPENED_BY_CLIENT.0),
            reader,
        }
    }

    pub(crate) async fn request(
        &self,
        method: Method,
        params: &impl Serialize,
    ) -> Result<Value, ClientError> {
        let id = self.next_id();
        let request = Request::new(id, method, params)?;
        let body = Message::from(request).encode()?;

        let ticket = self.pending.register(id)?;
        self.send(Frame::message(body)).await?;
        let outcome = ticket.reply().await?;

        Ok(outcome.into_result()?)
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<Notification> {
        match self.notifications.upgrade() {
            Some(notifications) => notifications.subscribe(),
            None => broadcast::channel(1).1,
        }
    }

    pub(crate) fn claim(&self, stream: StreamId) -> StreamReceiver {
        self.streams.claim(stream)
    }

    pub(crate) fn open_stream(&self) -> StreamReceiver {
        let stream = StreamId(self.next_stream.fetch_add(1, Ordering::Relaxed));

        self.streams.claim(stream)
    }

    pub(crate) async fn send_stream(
        &self,
        stream: StreamId,
        data: Bytes,
    ) -> Result<(), ClientError> {
        for start in (0..data.len()).step_by(STREAM_CHUNK) {
            let end = (start + STREAM_CHUNK).min(data.len());
            let chunk = data.slice(start..end);

            self.send(Frame::Stream {
                stream,
                data: chunk,
            })
            .await?;
        }

        Ok(())
    }

    fn next_id(&self) -> RequestId {
        RequestId(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    async fn send(&self, frame: Frame) -> Result<(), ClientError> {
        let sent = self.outgoing.lock().await.send(frame).await;

        match sent {
            Ok(()) => Ok(()),
            Err(FrameError::Io(_)) => {
                self.pending.close();
                Err(ClientError::Disconnected)
            }
            Err(error) => Err(ClientError::Frame(error)),
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.reader.abort();
    }
}
