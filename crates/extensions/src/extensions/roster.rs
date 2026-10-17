mod entry;

use std::collections::BTreeMap;
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use synt_actions::{ActionDescription, RegisteredAction};
use synt_journal::ActionId;

use entry::Entry;

use super::Loaded;
use crate::ExtensionError;
use crate::background::ExtensionEvent;

#[derive(Default)]
pub(crate) struct Roster {
    entries: RwLock<BTreeMap<String, Entry>>,
}

impl Roster {
    pub(crate) fn get(&self, id: &str) -> Option<Arc<Loaded>> {
        self.read().get(id).and_then(Entry::loaded).cloned()
    }

    pub(crate) fn all(&self) -> Vec<Arc<Loaded>> {
        self.read()
            .values()
            .filter_map(Entry::loaded)
            .cloned()
            .collect()
    }

    pub(crate) fn problem(&self, id: &str) -> Option<String> {
        self.read()
            .get(id)
            .and_then(Entry::problem)
            .map(str::to_owned)
    }

    pub(crate) fn put(&self, loaded: Arc<Loaded>) {
        let id = loaded.id().to_owned();

        self.write().insert(id, Entry::Loaded(loaded));
    }

    pub(crate) fn take(&self, id: &str) {
        self.write().remove(id);
    }

    pub(crate) fn note_problem(&self, id: &str, error: &ExtensionError) {
        eprintln!("synt-core: extension {id} is not loaded: {error}");

        self.write()
            .insert(id.to_owned(), Entry::Problem(error.to_string()));
    }

    pub(crate) fn wake_up(&self) {
        for loaded in self.all() {
            loaded.wake_up();
        }
    }

    pub(crate) fn publish(&self, event: &ExtensionEvent) -> usize {
        self.all()
            .iter()
            .filter(|loaded| loaded.hear(event))
            .count()
    }

    pub(crate) fn handle(&self, id: &ActionId) -> Option<RegisteredAction> {
        let (extension, _) = id.as_str().split_once('.')?;

        self.get(extension)?.handle(id)
    }

    pub(crate) fn describe(&self) -> Vec<ActionDescription> {
        self.all()
            .iter()
            .flat_map(|loaded| loaded.describe())
            .collect()
    }

    fn read(&self) -> RwLockReadGuard<'_, BTreeMap<String, Entry>> {
        self.entries.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write(&self) -> RwLockWriteGuard<'_, BTreeMap<String, Entry>> {
        self.entries.write().unwrap_or_else(PoisonError::into_inner)
    }
}
