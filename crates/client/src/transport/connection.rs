use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::SinkExt;
use serde::Serialize;
use serde_json::Value;
use synt_proto::{
    Frame, FrameCodec, FrameError, FrameWriter, Message, Method, Request, RequestId, split_framed,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use super::Duplex;
use super::pending::Pending;
use super::reader;
use crate::ClientError;

type Outgoing = FrameWriter<Box<dyn Duplex>>;

pub(crate) struct Connection {
    outgoing: Mutex<Outgoing>,
    pending: Arc<Pending>,
    next_id: AtomicU64,
    reader: JoinHandle<()>,
}

impl Connection {
    pub(crate) fn open(stream: impl Duplex + 'static) -> Self {
        let stream: Box<dyn Duplex> = Box::new(stream);
        let (incoming, outgoing) = split_framed(stream, FrameCodec::DEFAULT_LIMIT);

        let pending = Arc::new(Pending::new());
        let reader = tokio::spawn(reader::route(incoming, Arc::clone(&pending)));

        Self {
            outgoing: Mutex::new(outgoing),
            pending,
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
