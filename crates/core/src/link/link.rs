use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use bytes::Bytes;
use synt_proto::{Frame, StreamId};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;

type Routes = HashMap<StreamId, mpsc::Sender<Bytes>>;

pub(crate) struct Link {
    frames: mpsc::Sender<Frame>,
    routes: Mutex<Routes>,
    next_stream: AtomicU32,
}

impl Link {
    pub(crate) fn new(frames: mpsc::Sender<Frame>) -> Self {
        Self {
            frames,
            routes: Mutex::new(Routes::new()),
            next_stream: AtomicU32::new(1),
        }
    }

    pub(crate) fn frames(&self) -> mpsc::Sender<Frame> {
        self.frames.clone()
    }

    pub(crate) fn open_stream(&self, input: mpsc::Sender<Bytes>) -> StreamId {
        let stream = StreamId(self.next_stream.fetch_add(1, Ordering::Relaxed));
        self.lock().insert(stream, input);

        stream
    }

    pub(crate) fn forget(&self, stream: StreamId) {
        self.lock().remove(&stream);
    }

    pub(crate) fn deliver(&self, stream: StreamId, data: Bytes) {
        if data.is_empty() {
            return;
        }

        let Some(input) = self.lock().get(&stream).cloned() else {
            return;
        };

        if let Err(TrySendError::Closed(_)) = input.try_send(data) {
            self.forget(stream);
        }
    }

    fn lock(&self) -> MutexGuard<'_, Routes> {
        self.routes.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
