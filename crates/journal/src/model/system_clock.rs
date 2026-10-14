use std::time::{SystemTime, UNIX_EPOCH};

use super::{Clock, Timestamp};

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let millis = i64::try_from(since_epoch.as_millis()).unwrap_or(i64::MAX);

        Timestamp::from_millis(millis)
    }
}
