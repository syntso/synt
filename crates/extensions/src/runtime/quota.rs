use super::QuotaSpent;

const STORAGE_OPERATIONS: u32 = 1000;
const CALLS: u32 = 64;
const MESSAGES: u32 = 256;

pub(super) struct Quota {
    storage: u32,
    calls: u32,
    messages: u32,
}

impl Quota {
    pub(super) fn new() -> Self {
        Self {
            storage: STORAGE_OPERATIONS,
            calls: CALLS,
            messages: MESSAGES,
        }
    }

    pub(super) fn take_storage(&mut self) -> Result<(), QuotaSpent> {
        take(&mut self.storage).ok_or(QuotaSpent::new("storage operations", STORAGE_OPERATIONS))
    }

    pub(super) fn take_call(&mut self) -> Result<(), QuotaSpent> {
        take(&mut self.calls).ok_or(QuotaSpent::new("calls into the core", CALLS))
    }

    pub(super) fn take_message(&mut self) -> Result<(), QuotaSpent> {
        take(&mut self.messages).ok_or(QuotaSpent::new("messages to the app", MESSAGES))
    }
}

fn take(left: &mut u32) -> Option<()> {
    *left = left.checked_sub(1)?;

    Some(())
}
