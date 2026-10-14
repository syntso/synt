use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;
use wasmtime::Trap;

use super::QuotaSpent;
use crate::ExtensionError;

pub(crate) struct Budget {
    started: Instant,
    outside: Duration,
    limit: Duration,
    stop: CancellationToken,
}

impl Budget {
    pub(crate) fn new(limit: Duration, stop: CancellationToken) -> Self {
        Self {
            started: Instant::now(),
            outside: Duration::ZERO,
            limit,
            stop,
        }
    }

    pub(crate) fn stopped(&self) -> bool {
        self.stop.is_cancelled()
    }

    pub(crate) fn spent(&self) -> bool {
        let inside = self.started.elapsed().saturating_sub(self.outside);

        inside > self.limit
    }

    pub(crate) fn blame(&self, error: &wasmtime::Error) -> ExtensionError {
        if let Some(spent) = error.downcast_ref::<QuotaSpent>() {
            return ExtensionError::OverQuota(spent.to_string());
        }

        match error.downcast_ref::<Trap>() {
            Some(Trap::OutOfFuel) => ExtensionError::OutOfFuel,
            Some(Trap::Interrupt) if self.stopped() => ExtensionError::Stopped,
            Some(Trap::Interrupt) => ExtensionError::TimedOut(self.limit),
            _ => ExtensionError::Trapped(format!("{error:#}")),
        }
    }

    pub(crate) fn outside<T>(&mut self, work: impl FnOnce() -> T) -> T {
        let left = Instant::now();
        let result = work();

        self.outside += left.elapsed();
        result
    }
}
