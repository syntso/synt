use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;

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

    pub(crate) fn limit(&self) -> Duration {
        self.limit
    }

    pub(crate) fn stopped(&self) -> bool {
        self.stop.is_cancelled()
    }

    pub(crate) fn spent(&self) -> bool {
        let inside = self.started.elapsed().saturating_sub(self.outside);

        inside > self.limit
    }

    pub(crate) fn outside<T>(&mut self, work: impl FnOnce() -> T) -> T {
        let left = Instant::now();
        let result = work();

        self.outside += left.elapsed();
        result
    }
}
