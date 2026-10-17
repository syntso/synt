mod spend;

pub(crate) use spend::Spend;

use crate::ExtensionError;

pub(super) struct Quota {
    calls: u32,
    storage: u32,
    messages: u32,
}

impl Quota {
    pub(super) fn new() -> Self {
        Self {
            calls: Spend::Call.most(),
            storage: Spend::Storage.most(),
            messages: Spend::Message.most(),
        }
    }

    pub(super) fn take(&mut self, spend: Spend) -> Result<(), ExtensionError> {
        let Some(left) = self.counter(spend) else {
            return Ok(());
        };

        *left = left
            .checked_sub(1)
            .ok_or(ExtensionError::OverQuota(spend))?;

        Ok(())
    }

    fn counter(&mut self, spend: Spend) -> Option<&mut u32> {
        match spend {
            Spend::Call => Some(&mut self.calls),
            Spend::Storage => Some(&mut self.storage),
            Spend::Message => Some(&mut self.messages),
            Spend::Free => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Quota, Spend};

    #[test]
    fn each_kind_runs_out_on_its_own() {
        let mut quota = Quota::new();

        for _ in 0..Spend::Call.most() {
            quota.take(Spend::Call).unwrap();
        }

        assert!(quota.take(Spend::Call).is_err());
        assert!(quota.take(Spend::Storage).is_ok());
        assert!(quota.take(Spend::Free).is_ok());
    }
}
