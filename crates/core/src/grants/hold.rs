use std::sync::Arc;

use super::GrantId;
use super::holders::Holders;

#[derive(Debug)]
pub(crate) struct Hold {
    grant: GrantId,
    number: u64,
    holders: Arc<Holders>,
}

impl Hold {
    pub(super) fn new(grant: GrantId, number: u64, holders: Arc<Holders>) -> Self {
        Self {
            grant,
            number,
            holders,
        }
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        self.holders.release(self.grant, self.number);
    }
}
