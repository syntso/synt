use std::future::Future;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[derive(Debug, Default)]
pub(crate) struct ForwardTraffic {
    connections: AtomicU64,
    active: AtomicUsize,
    sent: AtomicU64,
    received: AtomicU64,
    last_error: Mutex<Option<String>>,
}

impl ForwardTraffic {
    pub(crate) async fn track(&self, connection: impl Future<Output = Result<(), String>>) {
        self.connections.fetch_add(1, Ordering::Relaxed);
        self.active.fetch_add(1, Ordering::Relaxed);

        let outcome = connection.await;

        self.active.fetch_sub(1, Ordering::Relaxed);

        if let Err(error) = outcome {
            self.note(error);
        }
    }

    pub(crate) fn add_sent(&self, bytes: usize) {
        self.sent.fetch_add(widen(bytes), Ordering::Relaxed);
    }

    pub(crate) fn add_received(&self, bytes: usize) {
        self.received.fetch_add(widen(bytes), Ordering::Relaxed);
    }

    pub(crate) fn note(&self, error: String) {
        if let Ok(mut last) = self.last_error.lock() {
            *last = Some(error);
        }
    }

    pub(crate) fn connections(&self) -> u64 {
        self.connections.load(Ordering::Relaxed)
    }

    pub(crate) fn active(&self) -> usize {
        self.active.load(Ordering::Relaxed)
    }

    pub(crate) fn sent(&self) -> u64 {
        self.sent.load(Ordering::Relaxed)
    }

    pub(crate) fn received(&self) -> u64 {
        self.received.load(Ordering::Relaxed)
    }

    pub(crate) fn last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|last| last.clone())
    }
}

fn widen(bytes: usize) -> u64 {
    u64::try_from(bytes).unwrap_or(u64::MAX)
}
