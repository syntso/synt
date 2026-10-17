use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard};

use synt_platform::Lock;

use crate::access::AccessRequest;
use crate::grants::GrantScope;

#[derive(Debug, Default)]
pub(super) struct UntilRestart {
    scopes: Mutex<HashSet<GrantScope>>,
}

impl UntilRestart {
    pub(super) fn covers(&self, request: &AccessRequest<'_>) -> bool {
        self.lock().contains(&GrantScope::covering(request))
    }

    pub(super) fn give(&self, scope: GrantScope) {
        self.lock().insert(scope);
    }

    fn lock(&self) -> MutexGuard<'_, HashSet<GrantScope>> {
        self.scopes.locked()
    }
}
