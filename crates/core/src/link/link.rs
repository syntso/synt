use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

use bytes::Bytes;
use synt_proto::{Frame, StreamId};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;

use super::{Delivery, Route};
use crate::locked::locked;

type Routes = HashMap<StreamId, Route>;

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

    pub(crate) fn detached() -> Self {
        let (frames, _) = mpsc::channel(1);

        Self::new(frames)
    }

    pub(crate) fn frames(&self) -> mpsc::Sender<Frame> {
        self.frames.clone()
    }

    pub(crate) fn open_stream(&self, input: mpsc::Sender<Bytes>) -> StreamId {
        let stream = StreamId(self.next_stream.fetch_add(1, Ordering::Relaxed));
        let route = Route {
            input,
            delivery: Delivery::Lossy,
        };
        self.lock().insert(stream, route);

        stream
    }

    pub(crate) fn claim(&self, stream: StreamId, route: Route) -> bool {
        if !stream.is_opened_by_client() {
            return false;
        }

        match self.lock().entry(stream) {
            Entry::Occupied(_) => false,
            Entry::Vacant(free) => {
                free.insert(route);
                true
            }
        }
    }

    pub(crate) fn forget(&self, stream: StreamId) {
        self.lock().remove(&stream);
    }

    pub(crate) fn close(&self) {
        let closed: Vec<Route> = self.lock().drain().map(|(_, route)| route).collect();
        drop(closed);
    }

    pub(crate) fn deliver(&self, stream: StreamId, data: Bytes) {
        if data.is_empty() {
            return;
        }

        let mut routes = self.lock();

        let Some(route) = routes.get_mut(&stream) else {
            return;
        };

        if !route.delivery.admits(data.len()) {
            routes.remove(&stream);
            return;
        }

        let broken = match route.input.try_send(data) {
            Ok(()) => false,
            Err(TrySendError::Closed(_)) => true,
            Err(TrySendError::Full(_)) => !route.delivery.drops_when_full(),
        };

        if broken {
            routes.remove(&stream);
        }
    }

    fn lock(&self) -> MutexGuard<'_, Routes> {
        locked(&self.routes)
    }
}
