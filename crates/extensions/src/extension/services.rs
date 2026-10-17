mod core_link;

use std::sync::{Arc, OnceLock};

use synt_journal::{Clock, Timestamp};
use synt_sync::OpenProjects;

pub(crate) use core_link::CoreLink;

use crate::http::SecretSource;
use crate::shelf::Shelf;

pub(crate) struct Services {
    shelf: Shelf,
    clock: Arc<dyn Clock>,
    secrets: OnceLock<Arc<dyn SecretSource>>,
    folders: OnceLock<Arc<dyn OpenProjects>>,
    link: OnceLock<CoreLink>,
}

impl Services {
    pub(crate) fn new(shelf: Shelf, clock: Arc<dyn Clock>) -> Self {
        Self {
            shelf,
            clock,
            secrets: OnceLock::new(),
            folders: OnceLock::new(),
            link: OnceLock::new(),
        }
    }

    pub(crate) fn shelf(&self) -> &Shelf {
        &self.shelf
    }

    pub(crate) fn now(&self) -> Timestamp {
        self.clock.now()
    }

    pub(crate) fn secrets(&self) -> Option<&Arc<dyn SecretSource>> {
        self.secrets.get()
    }

    pub(crate) fn folders(&self) -> Option<&dyn OpenProjects> {
        self.folders.get().map(AsRef::as_ref)
    }

    pub(crate) fn link(&self) -> Option<&CoreLink> {
        self.link.get()
    }

    pub(crate) fn give_secrets(&self, secrets: Arc<dyn SecretSource>) {
        Self::set_once(&self.secrets, secrets, "secrets");
    }

    pub(crate) fn give_folders(&self, folders: Arc<dyn OpenProjects>) {
        Self::set_once(&self.folders, folders, "project folders");
    }

    pub(crate) fn connect(&self, link: CoreLink) {
        Self::set_once(&self.link, link, "connection to the core");
    }

    fn set_once<T>(slot: &OnceLock<T>, value: T, what: &str) {
        if slot.set(value).is_err() {
            eprintln!("synt-core: extensions: the {what} were given twice; keeping the first");
        }
    }
}
