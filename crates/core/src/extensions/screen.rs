use std::sync::{Arc, Weak};

use synt_extensions::{Screen, ScreenUpdate};
use synt_proto::{Notice, Notification, PanelPosted, ViewPatched};

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

    fn notification(update: ScreenUpdate) -> Result<Notification, serde_json::Error> {
        match update {
            ScreenUpdate::ViewPatched {
                extension,
                view,
                patch,
            } => ViewPatched {
                extension,
                view,
                patch,
            }
            .to_notification(),
            ScreenUpdate::PanelPosted {
                extension,
                panel,
                message,
            } => PanelPosted {
                extension,
                panel,
                message,
            }
            .to_notification(),
        }
    }
}

impl Screen for CoreScreen {
    fn show(&self, update: ScreenUpdate) {
        let Some(core) = self.core.upgrade() else {
            return;
        };

        let notification = match Self::notification(update) {
            Ok(notification) => notification,
            Err(error) => {
                eprintln!("synt-core: an extension update cannot be encoded: {error}");
                return;
            }
        };

        core.broker().tell_apps(&notification);
    }
}
