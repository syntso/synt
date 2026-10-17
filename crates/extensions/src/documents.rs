mod info;
mod merger;
mod opened;
mod path;
mod pending;
mod place;
mod walk;
mod written;

pub(crate) use info::DocumentInfo;
pub use merger::MergeSides;
pub(crate) use merger::Merger;
pub(crate) use opened::Opened;
pub(crate) use written::Written;

use path::DocumentPath;
use pending::Pending;
use place::{DocumentPlace, LARGEST_DOCUMENT};
use synt_sync::{OpenProjects, ProjectFolder};
use walk::Walk;

use crate::call::FailureCode;
use crate::interface::GuestError;
use crate::manifest::Manifest;
use crate::sha256::Sha256;
use crate::shelf::{Revisions, StoredRevision};

pub(crate) struct DocumentStore<'a> {
    manifest: &'a Manifest,
    revisions: &'a Revisions,
    folders: &'a dyn OpenProjects,
}

impl<'a> DocumentStore<'a> {
    pub(crate) fn new(
        manifest: &'a Manifest,
        revisions: &'a Revisions,
        folders: &'a dyn OpenProjects,
    ) -> Self {
        Self {
            manifest,
            revisions,
            folders,
        }
    }

    pub(crate) fn list(&self, project: &str) -> Result<Vec<DocumentInfo>, GuestError> {
        let folder = self.project(project)?;
        let root = DocumentPlace::root(&folder, self.extension());

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

    pub(crate) fn read(&self, project: &str, path: &str) -> Result<Opened, GuestError> {
        let place = self.locate(project, path)?;
        let body = place.read()?.ok_or_else(|| place.not_found())?;

        self.note_outside_change(&place, &body)?;

        Ok(Opened::new(place.info(&body), body))
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
        Self::check_size(body.len())?;

        let pending = match place.read()? {
            Some(current) => self.onto_current(&place, &current, body, base, merger)?,
            None => Pending::plain(body, base.map(Sha256::recorded)),
        };

        place.write(pending.body())?;
        self.remember(&place, &pending.revision())?;

        Ok(Written::new(
            place.info(pending.body()),
            pending.was_merged(),
        ))
    }

    pub(crate) fn remove(&self, project: &str, path: &str) -> Result<(), GuestError> {
        let place = self.locate(project, path)?;

        place.remove()?;
        self.revisions
            .forget(place.project_id(), &place.history_key())?;

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

        self.merged(
            place,
            base,
            MergeSides::new(None, current, incoming),
            merger,
        )
    }

    fn merged(
        &self,
        place: &DocumentPlace,
        base: Option<&str>,
        sides: MergeSides<'_>,
        merger: &impl Merger,
    ) -> Result<Pending, GuestError> {
        let base_body = base
            .map(|base| self.known_revision(place, base))
            .transpose()?;
        let sides = MergeSides::new(base_body.as_deref(), sides.ours(), sides.theirs());

        let body = merger.merge(place.kind(), sides)?;
        Self::check_size(body.len())?;

        let parents = base.map(Sha256::recorded).into_iter().collect();
        let incoming = StoredRevision::new(sides.theirs(), parents);
        self.remember(place, &incoming)?;

        let parents = vec![Sha256::of(sides.ours()), incoming.revision().clone()];

        Ok(Pending::merged(body, parents))
    }

    fn note_outside_change(&self, place: &DocumentPlace, body: &[u8]) -> Result<(), GuestError> {
        let revision = Sha256::of(body);
        let head = self
            .revisions
            .head(place.project_id(), &place.history_key())?;

        if head.as_ref() == Some(&revision) {
            return Ok(());
        }

        let outside = StoredRevision::new(body, head.into_iter().collect());

        self.remember(place, &outside)
    }

    fn known_revision(&self, place: &DocumentPlace, revision: &str) -> Result<Vec<u8>, GuestError> {
        let stored = self
            .revisions
            .find(place.project_id(), &place.history_key(), revision)?;

        stored.map(StoredRevision::into_body).ok_or_else(|| {
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
        self.revisions
            .add(place.project_id(), &place.history_key(), revision)?;

        Ok(())
    }

    fn locate(&self, project: &str, path: &str) -> Result<DocumentPlace, GuestError> {
        let folder = self.project(project)?;

        self.place(folder, path)
    }

    fn project(&self, project: &str) -> Result<ProjectFolder, GuestError> {
        if !self.manifest.capabilities().documents() {
            return Err(GuestError::missing_capability(
                self.extension(),
                "documents = true",
            ));
        }

        self.folders
            .folder(project)
            .map_err(|reason| GuestError::new(FailureCode::UnknownProject, reason.to_string()))
    }

    fn place(&self, project: ProjectFolder, path: &str) -> Result<DocumentPlace, GuestError> {
        let path = DocumentPath::parse(path)?;

        let kind = self.manifest.document_kind(path.suffix()).ok_or_else(|| {
            GuestError::not_allowed(format!(
                "{} owns no document type with the suffix .{}",
                self.extension(),
                path.suffix()
            ))
        })?;

        Ok(DocumentPlace::new(project, self.extension(), path, kind))
    }

    fn extension(&self) -> &str {
        self.manifest.id().as_str()
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
}
