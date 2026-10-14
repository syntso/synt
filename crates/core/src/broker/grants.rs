use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard};

use super::grant::Grant;
use crate::locked::locked;

#[derive(Debug, Default)]
pub(super) struct Grants {
    standing: Mutex<HashSet<Grant>>,
}

impl Grants {
    pub(super) fn covers(&self, grant: &Grant) -> bool {
        self.lock().contains(grant)
    }

    pub(super) fn give(&self, grant: Grant) {
        self.lock().insert(grant);
    }

    fn lock(&self) -> MutexGuard<'_, HashSet<Grant>> {
        locked(&self.standing)
    }
}
