use std::sync::Arc;

use super::Visit;
use crate::documents::{Document, DocumentInfo, DocumentStore, Merger, Written};
use crate::extension::Guest;
use crate::interface::GuestError;

impl Visit {
    pub(super) fn list_documents(&self, project: &str) -> Result<Vec<DocumentInfo>, GuestError> {
        self.with_store(|store| store.list(project))
    }

    pub(super) fn open_document(&self, project: &str, path: &str) -> Result<Document, GuestError> {
        self.with_store(|store| store.read(project, path))
    }

    pub(super) fn save_document(
        &self,
        project: &str,
        path: &str,
        body: &[u8],
        base: Option<&str>,
    ) -> Result<Written, GuestError> {
        self.with_store(|store| store.write(project, path, body, base, &self.guest))
    }

    pub(super) fn delete_document(&self, project: &str, path: &str) -> Result<(), GuestError> {
        self.with_store(|store| store.remove(project, path))
    }

    fn with_store<T>(
        &self,
        work: impl FnOnce(&DocumentStore<'_>) -> Result<T, GuestError>,
    ) -> Result<T, GuestError> {
        let services = self.guest.services();
        let folders = services.folders.get().ok_or_else(|| {
            GuestError::new(
                "unavailable",
                "the core has no projects to keep documents in",
            )
        })?;

        let store = DocumentStore::new(self.manifest(), &services.shelf, folders.as_ref());

        work(&store)
    }
}

impl Merger for Arc<Guest> {
    fn merge(
        &self,
        kind: &str,
        sides: (Option<&[u8]>, &[u8], &[u8]),
    ) -> Result<Vec<u8>, GuestError> {
        let conflict =
            |reason: String| GuestError::new("conflict", format!("the merge failed: {reason}"));

        match Guest::merge(self, kind, sides) {
            Ok(Ok(merged)) => Ok(merged),
            Ok(Err(error)) => Err(conflict(error.to_string())),
            Err(error) => Err(conflict(error.to_string())),
        }
    }
}
