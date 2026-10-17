use std::sync::{Arc, OnceLock};

use synt_actions::{ActionDescription, Handle, Risk};
use synt_journal::ActionId;

use super::{ActionSpec, ExtensionAction, Guest};
use crate::background::{ExtensionEvent, Lane, Wake};
use crate::manifest::Api;
use crate::shelf::Installed;

pub(crate) struct Loaded {
    pub(crate) guest: Arc<Guest>,
    pub(crate) installed: Installed,
    pub(crate) ceiling: Risk,
    handles: Vec<Handle>,
    lane: OnceLock<Lane>,
}

impl Loaded {
    pub(crate) fn new(
        guest: Arc<Guest>,
        specs: Vec<ActionSpec>,
        ceiling: Risk,
        installed: Installed,
    ) -> Self {
        let handles = specs
            .into_iter()
            .map(|spec| {
                let id = spec.id.clone();
                Handle::dynamic(id, ExtensionAction::new(spec, Arc::clone(&guest)))
            })
            .collect();

        let loaded = Self {
            guest,
            installed,
            ceiling,
            handles,
            lane: OnceLock::new(),
        };

        loaded.wake_up();
        loaded
    }

    pub(crate) fn wake_up(&self) {
        if self.guest.manifest().api == Api::V0_1 {
            return;
        }

        let services = self.guest.services();
        let Some(handle) = services.background.get() else {
            return;
        };

        if services.caller.get().is_none() {
            return;
        }

        self.lane.get_or_init(|| {
            let lane = Lane::start(Arc::clone(&self.guest), handle);
            lane.wake(Wake::Start);
            lane
        });
    }

    pub(crate) fn handle(&self, id: &ActionId) -> Option<Handle> {
        self.handles
            .iter()
            .find(|handle| handle.id() == id)
            .cloned()
    }

    pub(crate) fn describe(&self) -> Vec<ActionDescription> {
        self.handles.iter().map(Handle::describe).collect()
    }

    pub(crate) fn action_ids(&self) -> Vec<String> {
        self.handles
            .iter()
            .map(|handle| handle.id().as_str().to_owned())
            .collect()
    }

    pub(crate) fn hear(&self, event: &ExtensionEvent) -> bool {
        let listens = self.guest.manifest().capabilities.hears(event.topic);

        if !listens || event.caused_by_an_extension() {
            return false;
        }

        self.lane
            .get()
            .is_some_and(|lane| lane.wake(Wake::Event(event.clone())))
    }
}
