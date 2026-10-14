use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::SshError;
use crate::locked::locked;

#[derive(Debug, Default)]
pub(crate) struct Traffic {
    sent: AtomicU64,
    received: AtomicU64,
    active: AtomicUsize,
    connections: AtomicU64,
    last_error: Mutex<Option<String>>,
}

impl Traffic {
    pub(crate) fn sent(&self) -> u64 {
        self.sent.load(Ordering::Relaxed)
    }

    pub(crate) fn received(&self) -> u64 {
        self.received.load(Ordering::Relaxed)
    }

    pub(crate) fn active(&self) -> usize {
        self.active.load(Ordering::Relaxed)
    }

    pub(crate) fn connections(&self) -> u64 {
        self.connections.load(Ordering::Relaxed)
    }

    pub(crate) fn last_error(&self) -> Option<String> {
        locked(&self.last_error).clone()
    }

    pub(crate) fn add_sent(&self, bytes: usize) {
        self.sent.fetch_add(widen(bytes), Ordering::Relaxed);
    }

    pub(crate) fn add_received(&self, bytes: usize) {
        self.received.fetch_add(widen(bytes), Ordering::Relaxed);
    }

    pub(crate) fn begin(&self) {
        self.active.fetch_add(1, Ordering::Relaxed);
        self.connections.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn end(&self, outcome: Result<(), SshError>) {
        self.active.fetch_sub(1, Ordering::Relaxed);

        if let Err(error) = outcome {
            self.note(&error);
        }
    }

    pub(crate) fn note(&self, error: &SshError) {
        *locked(&self.last_error) = Some(error.to_string());
    }
}

fn widen(bytes: usize) -> u64 {
    u64::try_from(bytes).unwrap_or(u64::MAX)
}
