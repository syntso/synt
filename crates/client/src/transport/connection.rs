use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::SinkExt;
use serde::Serialize;
use serde_json::Value;
use synt_proto::{
    Frame, FrameCodec, FrameError, FrameWriter, Message, Method, Notification, Request, RequestId,
    split_framed,
};
use tokio::sync::{Mutex, broadcast};
use tokio::task::JoinHandle;

use super::Duplex;
use super::pending::Pending;
use super::reader;
use crate::ClientError;

type Outgoing = FrameWriter<Box<dyn Duplex>>;

const NOTIFICATION_BACKLOG: usize = 64;

pub(crate) struct Connection {
    outgoing: Mutex<Outgoing>,
    pending: Arc<Pending>,
    notifications: broadcast::Sender<Notification>,
    next_id: AtomicU64,
    reader: JoinHandle<()>,
}

impl Connection {
    pub(crate) fn open(stream: impl Duplex + 'static) -> Self {
        let stream: Box<dyn Duplex> = Box::new(stream);
        let (incoming, outgoing) = split_framed(stream, FrameCodec::DEFAULT_LIMIT);

        let pending = Arc::new(Pending::new());
        let (notifications, _) = broadcast::channel(NOTIFICATION_BACKLOG);

        let routed = reader::route(incoming, Arc::clone(&pending), notifications.clone());
        let reader = tokio::spawn(routed);

        Self {
            outgoing: Mutex::new(outgoing),
            pending,
            notifications,
            next_id: AtomicU64::new(1),
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
        self.notifications.subscribe()
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
