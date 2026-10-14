use std::sync::Arc;

use super::GrantId;
use super::GrantUsers;

#[derive(Debug)]
pub(crate) struct Hold {
    grant: GrantId,
    number: u64,
    users: Arc<GrantUsers>,
}

impl Hold {
    pub(super) fn new(grant: GrantId, number: u64, users: Arc<GrantUsers>) -> Self {
        Self {
            grant,
            number,
            users,
        }
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        self.users.release(self.grant, self.number);
    }
}
