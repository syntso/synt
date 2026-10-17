use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use synt_platform::Lock;

use crate::SshError;

#[derive(Debug, Default)]
pub(super) struct Traffic {
    sent: AtomicU64,
    received: AtomicU64,
    active: AtomicUsize,
    connections: AtomicU64,
    last_error: Mutex<Option<String>>,
}

impl Traffic {
    pub(super) fn sent(&self) -> u64 {
        self.sent.load(Ordering::Relaxed)
    }

    pub(super) fn received(&self) -> u64 {
        self.received.load(Ordering::Relaxed)
    }

    pub(super) fn active(&self) -> usize {
        self.active.load(Ordering::Relaxed)
    }

    pub(super) fn connections(&self) -> u64 {
        self.connections.load(Ordering::Relaxed)
    }

    pub(super) fn last_error(&self) -> Option<String> {
        self.last_error.locked().clone()
    }

    pub(super) fn add_sent(&self, bytes: usize) {
        self.sent.fetch_add(Self::widen(bytes), Ordering::Relaxed);
    }

    pub(super) fn add_received(&self, bytes: usize) {
        self.received.fetch_add(Self::widen(bytes), Ordering::Relaxed);
    }

    pub(super) fn begin(&self) {
        self.active.fetch_add(1, Ordering::Relaxed);
        self.connections.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn end(&self, outcome: Result<(), SshError>) {
        self.active.fetch_sub(1, Ordering::Relaxed);

        if let Err(error) = outcome {
            self.note(&error);
        }
    }

    pub(super) fn note(&self, error: &SshError) {
        *self.last_error.locked() = Some(error.to_string());
    }

    fn widen(bytes: usize) -> u64 {
        u64::try_from(bytes).unwrap_or(u64::MAX)
    }
}
