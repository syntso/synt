use super::Slot;

pub(crate) struct DialAttempt<'a> {
    slot: &'a Slot,
    settled: bool,
}

impl<'a> DialAttempt<'a> {
    pub(crate) fn new(slot: &'a Slot) -> Self {
        Self {
            slot,
            settled: false,
        }
    }

    pub(crate) fn settle(mut self) {
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
