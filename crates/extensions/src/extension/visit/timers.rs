use super::Visit;
use crate::call::FailureCode;
use crate::interface::GuestError;

impl Visit {
    pub(super) fn arm_timer(&self, name: &str, every_seconds: u32) -> Result<(), GuestError> {
        self.allowed(self.manifest().capabilities.timers, "timers = true")?;

        self.guest
            .timers()
            .set(name, every_seconds)
            .map_err(|reason| GuestError::new(FailureCode::InvalidTimer, reason))
    }

    pub(super) fn disarm_timer(&self, name: &str) {
        self.guest.timers().clear(name);
    }
}
