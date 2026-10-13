use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::grant::Grant;

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
        self.standing.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
