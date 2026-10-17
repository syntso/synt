use std::sync::{Arc, Weak};

use synt_extensions::{Screen, ScreenUpdate};

use crate::core::Core;

pub struct CoreScreen {
    core: Weak<Core>,
}

impl CoreScreen {
    pub fn new(core: &Arc<Core>) -> Self {
        Self {
            core: Arc::downgrade(core),
        }
    }
}

impl Screen for CoreScreen {
    fn show(&self, update: ScreenUpdate) {
        let Some(core) = self.core.upgrade() else {
            return;
        };

        let notification = update.to_notification();

        match notification {
            Ok(notification) => {
                core.broker().tell_apps(&notification);
            }
            Err(error) => eprintln!("synt-core: an extension update cannot be encoded: {error}"),
        }
    }
}
