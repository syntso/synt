use std::future;
use std::time::Duration;

use tokio::time::{self, Instant};

use super::Activity;

const LOOK_EVERY: Duration = Duration::from_secs(5);

pub struct IdleWatch {
    limit: Option<Duration>,
    sources: Vec<Box<dyn Activity>>,
}

impl IdleWatch {
    pub fn new(limit: Option<Duration>) -> Self {
        Self {
            limit,
            sources: Vec::new(),
        }
    }

    #[must_use]
    pub fn watch(mut self, source: impl Activity + 'static) -> Self {
        self.sources.push(Box::new(source));
        self
    }

    pub async fn idle(self) {
        let Some(limit) = self.limit else {
            return future::pending().await;
        };

        let mut quiet_since = Instant::now();
        let mut ticks = time::interval(LOOK_EVERY.min(limit).max(Duration::from_secs(1)));

        loop {
            let now = ticks.tick().await;

            if self.is_active() {
                quiet_since = now;
                continue;
            }

            if now.duration_since(quiet_since) >= limit {
                return;
            }
        }
    }

    fn is_active(&self) -> bool {
        self.sources.iter().any(|source| source.is_active())
    }
}
