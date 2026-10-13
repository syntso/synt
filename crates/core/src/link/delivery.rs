use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) enum Delivery {
    Lossy,
    Strict,
    Metered {
        allowance: Arc<AtomicU64>,
        received: u64,
    },
}

impl Delivery {
    pub(crate) fn admits(&mut self, length: usize) -> bool {
        let Self::Metered {
            allowance,
            received,
        } = self
        else {
            return true;
        };

        let after = received.saturating_add(u64::try_from(length).unwrap_or(u64::MAX));

        if after > allowance.load(Ordering::Acquire) {
            return false;
        }

        *received = after;
        true
    }

    pub(crate) fn drops_when_full(&self) -> bool {
        matches!(self, Self::Lossy)
    }
}
