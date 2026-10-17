use std::sync::Arc;

use crate::call::FailureCode;
use crate::extension::Guest;
use crate::interface::GuestError;

pub(super) struct TimerAccess {
    guest: Arc<Guest>,
}

impl TimerAccess {
    pub(super) fn new(guest: Arc<Guest>) -> Self {
        Self { guest }
    }

    pub(super) fn set(&self, name: &str, every_seconds: u32) -> Result<(), GuestError> {
        let timers = self.guest.manifest().capabilities().timers();
        self.guest.require(timers, "timers = true")?;

        self.guest
            .timers()
            .set(name, every_seconds)
            .map_err(|reason| GuestError::new(FailureCode::InvalidTimer, reason.to_string()))
    }

    pub(super) fn clear(&self, name: &str) {
        self.guest.timers().clear(name);
    }
}
