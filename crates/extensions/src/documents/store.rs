use super::path::DocumentPath;
use super::place::{DocumentPlace, LARGEST_DOCUMENT};
use super::walk::Walk;
use super::{Document, DocumentInfo, Merger, ProjectFolder, ProjectFolders, Written};
use crate::call::FailureCode;
use crate::interface::GuestError;
use crate::manifest::Manifest;
use crate::sha256::Sha256;
use crate::shelf::{Shelf, StoredRevision};

pub(crate) struct DocumentStore<'a> {
    manifest: &'a Manifest,
    shelf: &'a Shelf,
    folders: &'a dyn ProjectFolders,
}

struct Pending {
    body: Vec<u8>,
    parents: Vec<Sha256>,
    merged: bool,
}

impl<'a> DocumentStore<'a> {
    pub(crate) fn new(
        manifest: &'a Manifest,
        shelf: &'a Shelf,
        folders: &'a dyn ProjectFolders,
    ) -> Self {
        Self {
            manifest,
            shelf,
            folders,
        }
    }

    pub(crate) fn list(&self, project: &str) -> Result<Vec<DocumentInfo>, GuestError> {
        let folder = self.project(project)?;
        let root = DocumentPlace::root(&folder, &self.manifest.id);

        let mut documents = Vec::new();

        for relative in Walk::files(&root)? {
            if let Ok(place) = self.place(folder.clone(), &relative)
                && let Some(body) = place.read()?
            {
                documents.push(place.info(&body));
            }
        }

        Ok(documents)
    }

    pub(crate) fn read(&self, project: &str, path: &str) -> Result<Document, GuestError> {
        let place = self.locate(project, path)?;
        let body = place.read()?.ok_or_else(|| place.not_found())?;

        self.note_outside_change(&place, &body)?;

        Ok(Document {
            info: place.info(&body),
            body,
        })
    }

    pub(crate) fn write(
        &self,
        project: &str,
        path: &str,
        body: &[u8],
        base: Option<&str>,
        merger: &impl Merger,
    ) -> Result<Written, GuestError> {
        let place = self.locate(project, path)?;
        check_size(body.len())?;

        let pending = match place.read()? {
            Some(current) => self.onto_current(&place, &current, body, base, merger)?,
            None => Pending::plain(body, base.map(Sha256::recorded)),
        };

        place.write(&pending.body)?;
        self.remember(&place, &StoredRevision::new(&pending.body, pending.parents))?;

        Ok(Written {
            info: place.info(&pending.body),
            merged: pending.merged,
        })
    }

    pub(crate) fn remove(&self, project: &str, path: &str) -> Result<(), GuestError> {
        let place = self.locate(project, path)?;

        place.remove()?;
        self.shelf
            .forget_revisions(place.project_id(), &place.history_key())?;

        Ok(())
    }

    fn onto_current(
        &self,
        place: &DocumentPlace,
        current: &[u8],
        incoming: &[u8],
        base: Option<&str>,
        merger: &impl Merger,
    ) -> Result<Pending, GuestError> {
        self.note_outside_change(place, current)?;

        let revision = Sha256::of(current);
        if base == Some(revision.as_str()) {
            return Ok(Pending::plain(incoming, Some(revision)));
        }

        self.merged(place, base, current, revision, incoming, merger)
    }

    fn merged(
        &self,
        place: &DocumentPlace,
        base: Option<&str>,
        current: &[u8],
        current_revision: Sha256,
        incoming: &[u8],
        merger: &impl Merger,
    ) -> Result<Pending, GuestError> {
        let base_body = base
            .map(|base| self.known_revision(place, base))
            .transpose()?;

        let body = merger.merge(place.kind(), (base_body.as_deref(), current, incoming))?;
        check_size(body.len())?;

        let incoming =
            StoredRevision::new(incoming, base.map(Sha256::recorded).into_iter().collect());
        self.remember(place, &incoming)?;

        Ok(Pending {
            body,
            parents: vec![current_revision, incoming.revision],
            merged: true,
        })
    }

    fn note_outside_change(&self, place: &DocumentPlace, body: &[u8]) -> Result<(), GuestError> {
        let revision = Sha256::of(body);
        let head = self.shelf.head(place.project_id(), &place.history_key())?;

        if head.as_ref() == Some(&revision) {
            return Ok(());
        }

        let outside = StoredRevision::new(body, head.into_iter().collect());

        self.remember(place, &outside)
    }

    fn known_revision(&self, place: &DocumentPlace, revision: &str) -> Result<Vec<u8>, GuestError> {
        let stored = self
            .shelf
            .revision(place.project_id(), &place.history_key(), revision)?;

        stored.map(|stored| stored.body).ok_or_else(|| {
            GuestError::new(
                FailureCode::Conflict,
                format!(
                    "{} changed and its base revision {revision} is no longer known; \
                     read it again",
                    place.path()
                ),
            )
        })
    }

    fn remember(&self, place: &DocumentPlace, revision: &StoredRevision) -> Result<(), GuestError> {
        self.shelf
            .add_revision(place.project_id(), &place.history_key(), revision)?;

        Ok(())
    }

    fn locate(&self, project: &str, path: &str) -> Result<DocumentPlace, GuestError> {
        let folder = self.project(project)?;

        self.place(folder, path)
    }

    fn project(&self, project: &str) -> Result<ProjectFolder, GuestError> {
        if !self.manifest.capabilities.documents {
            return Err(GuestError::not_allowed(format!(
                "{} has no documents = true in its capabilities",
                self.manifest.id
            )));
        }

        self.folders
            .folder(project)
            .map_err(|reason| GuestError::new(FailureCode::UnknownProject, reason))
    }

    fn place(&self, project: ProjectFolder, path: &str) -> Result<DocumentPlace, GuestError> {
        let path = DocumentPath::parse(path)
            .map_err(|reason| GuestError::new(FailureCode::InvalidPath, reason))?;

        let kind = self.manifest.document_kind(path.suffix()).ok_or_else(|| {
            GuestError::not_allowed(format!(
                "{} owns no document type with the suffix .{}",
                self.manifest.id,
                path.suffix()
            ))
        })?;

        Ok(DocumentPlace::new(project, &self.manifest.id, path, kind))
    }
}

impl Pending {
    fn plain(body: &[u8], parent: Option<Sha256>) -> Self {
        Self {
            body: body.to_vec(),
            parents: parent.into_iter().collect(),
            merged: false,
        }
    }
}

fn check_size(size: usize) -> Result<(), GuestError> {
    if u64::try_from(size).unwrap_or(u64::MAX) <= LARGEST_DOCUMENT {
        return Ok(());
    }

    Err(GuestError::new(
        FailureCode::TooLarge,
        format!("a document holds at most {LARGEST_DOCUMENT} bytes"),
    ))
}
