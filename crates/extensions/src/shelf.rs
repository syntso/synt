mod count;
mod index_serial;
mod installed;
mod inventory;
mod items;
mod picks;
mod revisions;
mod schema;
mod sqlite;
mod stored_settings;

use std::path::Path;

pub(crate) use index_serial::IndexSerial;
pub(crate) use installed::Installed;
pub(crate) use inventory::Inventory;
pub(crate) use items::Items;
pub(crate) use picks::Picks;
pub(crate) use revisions::{Revisions, StoredRevision};
pub(crate) use stored_settings::StoredSettings;

use count::Count;
use sqlite::Sqlite;

use crate::ExtensionError;

#[derive(Debug, Clone)]
pub(crate) struct Shelf {
    inventory: Inventory,
    items: Items,
    picks: Picks,
    revisions: Revisions,
    settings: StoredSettings,
    index: IndexSerial,
}

impl Shelf {
    pub(crate) fn open(path: &Path) -> Result<Self, ExtensionError> {
        let sqlite = Sqlite::open(path)?;

        Ok(Self {
            inventory: Inventory::new(sqlite.clone()),
            items: Items::new(sqlite.clone()),
            picks: Picks::new(sqlite.clone()),
            revisions: Revisions::new(sqlite.clone()),
            settings: StoredSettings::new(sqlite.clone()),
            index: IndexSerial::new(sqlite),
        })
    }

    pub(crate) fn inventory(&self) -> &Inventory {
        &self.inventory
    }

    pub(crate) fn items(&self) -> &Items {
        &self.items
    }

    pub(crate) fn picks(&self) -> &Picks {
        &self.picks
    }

    pub(crate) fn revisions(&self) -> &Revisions {
        &self.revisions
    }

    pub(crate) fn settings(&self) -> &StoredSettings {
        &self.settings
    }

    pub(crate) fn index(&self) -> &IndexSerial {
        &self.index
    }
}

#[cfg(test)]
impl Shelf {
    pub(crate) fn scratch() -> (tempfile::TempDir, Self) {
        let folder = tempfile::tempdir().unwrap();
        let shelf = Self::open(&folder.path().join("state.sqlite")).unwrap();

        (folder, shelf)
    }
}
