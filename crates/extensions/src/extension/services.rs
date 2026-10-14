use std::sync::{Arc, OnceLock};

use tokio::runtime::Handle;

use crate::call::Caller;
use crate::documents::ProjectFolders;
use crate::http::SecretSource;
use crate::shelf::Shelf;
use crate::ui::Screen;

pub(crate) struct Services {
    pub(crate) shelf: Shelf,
    pub(crate) caller: OnceLock<Arc<dyn Caller>>,
    pub(crate) screen: OnceLock<Arc<dyn Screen>>,
    pub(crate) folders: OnceLock<Arc<dyn ProjectFolders>>,
    pub(crate) secrets: OnceLock<Arc<dyn SecretSource>>,
    pub(crate) background: OnceLock<Handle>,
}

impl Services {
    pub(crate) fn new(shelf: Shelf) -> Self {
        Self {
            shelf,
            caller: OnceLock::new(),
            screen: OnceLock::new(),
            folders: OnceLock::new(),
            secrets: OnceLock::new(),
            background: OnceLock::new(),
        }
    }
}
