use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use synt_proto::{Message, Notification};
use tokio::sync::mpsc;

use super::Registration;
use crate::locked::locked;

type Sessions = HashMap<u64, mpsc::Sender<Message>>;

#[derive(Debug, Default)]
pub(super) struct Apps {
    sessions: Mutex<Sessions>,
    next_id: AtomicU64,
}

impl Apps {
    pub(super) fn join(self: &Arc<Self>, outgoing: mpsc::Sender<Message>) -> Registration {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.lock().insert(id, outgoing);

        Registration::new(id, Arc::clone(self))
    }

    pub(super) fn leave(&self, id: u64) {
        self.lock().remove(&id);
    }

    pub(super) fn broadcast(&self, notification: &Notification) -> usize {
        let sessions = self.lock();

        sessions
            .values()
            .filter(|outgoing| {
                let message = Message::from(notification.clone());
                outgoing.try_send(message).is_ok()
            })
            .count()
    }

    fn lock(&self) -> MutexGuard<'_, Sessions> {
        locked(&self.sessions)
    }
}
