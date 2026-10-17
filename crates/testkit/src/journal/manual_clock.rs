use std::sync::atomic::{AtomicI64, Ordering};

use synt_journal::{Clock, Timestamp};

pub struct ManualClock {
    millis: AtomicI64,
}

impl ManualClock {
    pub fn starting_at(millis: i64) -> Self {
        Self {
            millis: AtomicI64::new(millis),
        }
    }

    pub fn set(&self, millis: i64) {
        self.millis.store(millis, Ordering::SeqCst);
    }

    pub fn advance(&self, millis: i64) {
        self.millis.fetch_add(millis, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_millis(self.millis.load(Ordering::SeqCst))
    }
}
