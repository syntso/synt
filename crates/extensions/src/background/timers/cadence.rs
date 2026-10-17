use std::time::Duration;

use tokio::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Cadence {
    every: Duration,
    next: Instant,
}

impl Cadence {
    pub(super) fn starting_now(every: Duration) -> Self {
        Self {
            every,
            next: Instant::now() + every,
        }
    }

    pub(super) fn next(&self) -> Instant {
        self.next
    }

    pub(super) fn is_due(&self, now: Instant) -> bool {
        self.next <= now
    }

    pub(super) fn advance(&mut self, now: Instant) {
        self.next = now + self.every;
    }
}
