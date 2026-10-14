use synt_journal::{Clock, SystemClock};

pub(crate) fn now() -> u64 {
    u64::try_from(SystemClock.now().as_millis()).unwrap_or_default()
}
