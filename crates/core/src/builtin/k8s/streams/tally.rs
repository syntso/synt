use std::time::Instant;

use super::{EndReason, StreamEnd};
use crate::clock::milliseconds;

pub(crate) struct Tally {
    started: Instant,
    bytes: u64,
    items: u64,
}

impl Tally {
    pub(crate) fn start() -> Self {
        Self {
            started: Instant::now(),
            bytes: 0,
            items: 0,
        }
    }

    pub(crate) fn add(&mut self, bytes: usize, items: usize) {
        self.bytes += u64::try_from(bytes).unwrap_or(u64::MAX);
        self.items += u64::try_from(items).unwrap_or(u64::MAX);
    }

    pub(crate) fn end(&self, reason: EndReason, error: Option<String>) -> StreamEnd {
        StreamEnd {
            reason,
            error,
            bytes: self.bytes,
            items: self.items,
            duration_ms: milliseconds(self.started.elapsed()),
        }
    }
}
