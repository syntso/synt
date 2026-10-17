use super::Slot;

pub(super) struct DialAttempt<'a> {
    slot: &'a Slot,
    settled: bool,
}

impl<'a> DialAttempt<'a> {
    pub(super) fn new(slot: &'a Slot) -> Self {
        Self {
            slot,
            settled: false,
        }
    }

    pub(super) fn settle(mut self) {
        self.settled = true;
    }
}

impl Drop for DialAttempt<'_> {
    fn drop(&mut self) {
        if !self.settled {
            self.slot.abandon_dial();
        }
    }
}
