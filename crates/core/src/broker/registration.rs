use std::sync::Arc;

use super::apps::Apps;

#[derive(Debug)]
pub(crate) struct Registration {
    id: u64,
    apps: Arc<Apps>,
}

impl Registration {
    pub(super) fn new(id: u64, apps: Arc<Apps>) -> Self {
        Self { id, apps }
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        self.apps.leave(self.id);
    }
}
