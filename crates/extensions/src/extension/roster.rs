use std::collections::BTreeMap;
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use synt_actions::{ActionDescription, Handle};
use synt_journal::ActionId;

use super::Loaded;
use crate::ExtensionError;
use crate::background::ExtensionEvent;

#[derive(Default)]
pub(crate) struct Roster {
    loaded: RwLock<BTreeMap<String, Arc<Loaded>>>,
    problems: RwLock<BTreeMap<String, String>>,
}

impl Roster {
    pub(crate) fn get(&self, id: &str) -> Option<Arc<Loaded>> {
        read(&self.loaded).get(id).cloned()
    }

    pub(crate) fn all(&self) -> Vec<Arc<Loaded>> {
        read(&self.loaded).values().cloned().collect()
    }

    pub(crate) fn problem(&self, id: &str) -> Option<String> {
        read(&self.problems).get(id).cloned()
    }

    pub(crate) fn put(&self, loaded: Arc<Loaded>) {
        let id = loaded.installed.id.clone();

        write(&self.problems).remove(&id);
        write(&self.loaded).insert(id, loaded);
    }

    pub(crate) fn take(&self, id: &str) {
        write(&self.loaded).remove(id);
        write(&self.problems).remove(id);
    }

    pub(crate) fn note_problem(&self, id: &str, error: &ExtensionError) {
        eprintln!("synt-core: extension {id} is not loaded: {error}");

        write(&self.problems).insert(id.to_owned(), error.to_string());
    }

    pub(crate) fn wake_up(&self) {
        for loaded in read(&self.loaded).values() {
            loaded.wake_up();
        }
    }

    pub(crate) fn publish(&self, event: &ExtensionEvent) -> usize {
        read(&self.loaded)
            .values()
            .filter(|loaded| loaded.hear(event))
            .count()
    }

    pub(crate) fn handle(&self, id: &ActionId) -> Option<Handle> {
        let (extension, _) = id.as_str().split_once('.')?;

        read(&self.loaded).get(extension)?.handle(id)
    }

    pub(crate) fn describe(&self) -> Vec<ActionDescription> {
        read(&self.loaded)
            .values()
            .flat_map(|loaded| loaded.describe())
            .collect()
    }
}

fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

fn write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}
