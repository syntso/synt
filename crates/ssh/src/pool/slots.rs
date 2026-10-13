use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{ConnectionStatus, Lease, Slot, locked};
use crate::Target;

pub(crate) struct Slots {
    slots: Mutex<HashMap<Target, Arc<Slot>>>,
    grace: Duration,
}

impl Slots {
    pub(crate) fn new(grace: Duration) -> Arc<Self> {
        Arc::new(Self {
            slots: Mutex::new(HashMap::new()),
            grace,
        })
    }

    pub(crate) fn grace(&self) -> Duration {
        self.grace
    }

    pub(crate) fn lease(self: &Arc<Self>, target: &Target) -> Lease {
        let mut slots = locked(&self.slots);

        let reusable = slots
            .get(target)
            .filter(|slot| !slot.has_failed())
            .map(Arc::clone);

        let slot = reusable.unwrap_or_else(|| {
            let fresh = Arc::new(Slot::new(target.clone(), Arc::downgrade(self)));
            slots.insert(target.clone(), Arc::clone(&fresh));
            fresh
        });

        Lease::new(slot)
    }

    pub(crate) fn retire(&self, slot: &Arc<Slot>, release: u64) -> bool {
        let mut slots = locked(&self.slots);

        if !slot.idle_since(release) {
            return false;
        }

        let current = slots
            .get(slot.target())
            .is_some_and(|entry| Arc::ptr_eq(entry, slot));

        if current {
            slots.remove(slot.target());
        }

        true
    }

    pub(crate) fn statuses(&self) -> Vec<ConnectionStatus> {
        let slots: Vec<Arc<Slot>> = locked(&self.slots).values().map(Arc::clone).collect();

        let mut statuses: Vec<ConnectionStatus> = slots.iter().map(|slot| slot.status()).collect();
        statuses.sort_by(|left, right| left.target.cmp(&right.target));

        statuses
    }
}
