use std::time::Duration;

use synt_journal::{Clock, SystemClock};

pub(crate) fn milliseconds(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

pub(crate) fn unix_millis() -> i64 {
    SystemClock.now().as_millis()
}
