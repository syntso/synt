use std::sync::Arc;

use synt_actions::{Description, Handle};
use synt_journal::ActionId;

use super::{ActionSpec, ExtensionAction, Guest};
use crate::shelf::Installed;

pub(crate) struct Loaded {
    pub(crate) guest: Arc<Guest>,
    pub(crate) installed: Installed,
    handles: Vec<Handle>,
}

impl Loaded {
    pub(crate) fn new(guest: Arc<Guest>, specs: Vec<ActionSpec>, installed: Installed) -> Self {
        let handles = specs
            .into_iter()
            .map(|spec| {
                let id = spec.id.clone();
                Handle::dynamic(id, ExtensionAction::new(spec, Arc::clone(&guest)))
            })
            .collect();

        Self {
            guest,
            installed,
            handles,
        }
    }

    pub(crate) fn handle(&self, id: &ActionId) -> Option<Handle> {
        self.handles
            .iter()
            .find(|handle| handle.id() == id)
            .cloned()
    }

    pub(crate) fn describe(&self) -> Vec<Description> {
        self.handles.iter().map(Handle::describe).collect()
    }

    pub(crate) fn action_ids(&self) -> Vec<String> {
        self.handles
            .iter()
            .map(|handle| handle.id().as_str().to_owned())
            .collect()
    }
}
