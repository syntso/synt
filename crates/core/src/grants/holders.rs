use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use tokio_util::sync::CancellationToken;

use super::{GrantId, Hold};
use crate::locked::locked;

type Held = HashMap<GrantId, HashMap<u64, CancellationToken>>;

#[derive(Debug, Default)]
pub(super) struct Holders {
    held: Mutex<Held>,
    next_number: AtomicU64,
}

impl Holders {
    pub(super) fn hold(self: &Arc<Self>, grant: GrantId, stop: CancellationToken) -> Hold {
        let number = self.next_number.fetch_add(1, Ordering::Relaxed);

        self.lock().entry(grant).or_default().insert(number, stop);

        Hold::new(grant, number, Arc::clone(self))
    }

    pub(super) fn stop_all(&self, grant: GrantId) -> usize {
        let stopped = self.lock().remove(&grant).unwrap_or_default();

        for stop in stopped.values() {
            stop.cancel();
        }

        stopped.len()
    }

    pub(super) fn release(&self, grant: GrantId, number: u64) {
        let mut held = self.lock();

        let Some(calls) = held.get_mut(&grant) else {
            return;
        };

        calls.remove(&number);

        if calls.is_empty() {
            held.remove(&grant);
        }
    }

    fn lock(&self) -> MutexGuard<'_, Held> {
        locked(&self.held)
    }
}
