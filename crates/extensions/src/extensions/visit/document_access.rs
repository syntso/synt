use std::sync::Arc;

use crate::call::FailureCode;
use crate::documents::{DocumentInfo, DocumentStore, MergeSides, Merger, Opened, Written};
use crate::extensions::Guest;
use crate::interface::GuestError;

pub(super) struct DocumentAccess {
    guest: Arc<Guest>,
}

impl DocumentAccess {
    pub(super) fn new(guest: Arc<Guest>) -> Self {
        Self { guest }
    }

    pub(super) fn list(&self, project: &str) -> Result<Vec<DocumentInfo>, GuestError> {
        self.with_store(|store| store.list(project))
    }

    pub(super) fn read(&self, project: &str, path: &str) -> Result<Opened, GuestError> {
        self.with_store(|store| store.read(project, path))
    }

    pub(super) fn write(
        &self,
        project: &str,
        path: &str,
        body: &[u8],
        base: Option<&str>,
    ) -> Result<Written, GuestError> {
        self.with_store(|store| store.write(project, path, body, base, &self.guest))
    }

    pub(super) fn remove(&self, project: &str, path: &str) -> Result<(), GuestError> {
        self.with_store(|store| store.remove(project, path))
    }

    fn with_store<T>(
        &self,
        work: impl FnOnce(&DocumentStore<'_>) -> Result<T, GuestError>,
    ) -> Result<T, GuestError> {
        let services = self.guest.services();
        let folders = services.folders().ok_or_else(|| {
            GuestError::new(
                FailureCode::Unavailable,
                "the core has no projects to keep documents in",
            )
        })?;

        let store =
            DocumentStore::new(self.guest.manifest(), services.shelf().revisions(), folders);

        work(&store)
    }
}

impl Merger for Arc<Guest> {
    fn merge(&self, kind: &str, sides: MergeSides<'_>) -> Result<Vec<u8>, GuestError> {
        Guest::merge(self, kind, sides).map_err(|error| {
            GuestError::new(FailureCode::Conflict, format!("the merge failed: {error}"))
        })
    }
}
