use std::sync::Arc;

use tokio::runtime::Handle;

use crate::call::Caller;
use crate::ui::{Screen, ScreenUpdate};

pub(crate) struct CoreLink {
    caller: Arc<dyn Caller>,
    screen: Arc<dyn Screen>,
    background: Option<Handle>,
}

impl CoreLink {
    pub(crate) fn new(caller: Arc<dyn Caller>, screen: Arc<dyn Screen>) -> Self {
        Self {
            caller,
            screen,
            background: Handle::try_current().ok(),
        }
    }

    pub(crate) fn caller(&self) -> &Arc<dyn Caller> {
        &self.caller
    }

    pub(crate) fn background(&self) -> Option<&Handle> {
        self.background.as_ref()
    }

    pub(crate) fn show(&self, update: ScreenUpdate) {
        self.screen.show(update);
    }
}
