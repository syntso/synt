use std::sync::Arc;

use super::Visit;
use crate::documents::{Document, DocumentInfo, DocumentStore, Written};
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
        let guest = Arc::clone(&self.guest);
        let mut merge = |kind: &str, base: Option<&[u8]>, ours: &[u8], theirs: &[u8]| match guest
            .merge(kind, (base, ours, theirs))
        {
            Ok(Ok(merged)) => Ok(merged),
            Ok(Err(error)) => Err(GuestError::new(
                "conflict",
                format!("the merge failed: {}: {}", error.code, error.message),
            )),
            Err(error) => Err(GuestError::new(
                "conflict",
                format!("the merge failed: {error}"),
            )),
        };

        self.with_store(|store| store.write(project, path, body, base, &mut merge))
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
