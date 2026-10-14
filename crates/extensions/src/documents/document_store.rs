use std::fs;
use std::io;
use std::path::Path;

use synt_platform::{read_plain_file, replace_file};

use super::{
    Document, DocumentInfo, DocumentPath, DocumentPlace, ProjectFolder, ProjectFolders, Written,
};
use crate::interface::GuestError;
use crate::manifest::Manifest;
use crate::package::sha256;
use crate::shelf::{Shelf, StoredRevision};

const LARGEST_DOCUMENT: u64 = 4 * 1024 * 1024;
const DEEPEST: usize = 8;
const MOST_LISTED: usize = 1_000;

pub(crate) type Merge<'m> =
    &'m mut dyn FnMut(&str, Option<&[u8]>, &[u8], &[u8]) -> Result<Vec<u8>, GuestError>;

pub(crate) struct DocumentStore<'a> {
    manifest: &'a Manifest,
    shelf: &'a Shelf,
    folders: &'a dyn ProjectFolders,
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

        let mut found = Vec::new();
        collect(&root, "", 0, &mut found).map_err(|error| failed(&error))?;

        let mut documents = Vec::new();
        for relative in found {
            if let Ok(place) = self.place(folder.clone(), &relative)
                && let Some(body) = read(&place)?
            {
                documents.push(info(&place, &body));
            }
        }

        Ok(documents)
    }

    pub(crate) fn read(&self, project: &str, path: &str) -> Result<Document, GuestError> {
        let place = self.place(self.project(project)?, path)?;
        let body = read(&place)?.ok_or_else(|| not_found(&place))?;

        self.note_outside_change(&place, &body)?;

        Ok(Document {
            info: info(&place, &body),
            body,
        })
    }

    pub(crate) fn write(
        &self,
        project: &str,
        path: &str,
        body: &[u8],
        base: Option<&str>,
        merge: Merge<'_>,
    ) -> Result<Written, GuestError> {
        let place = self.place(self.project(project)?, path)?;
        check_size(body.len())?;

        let current = read(&place)?;
        if let Some(current) = &current {
            self.note_outside_change(&place, current)?;
        }

        let current_revision = current.as_deref().map(sha256);
        let (body, parents, merged) = match (&current, &current_revision) {
            (Some(current), Some(revision)) if base != Some(revision.as_str()) => {
                self.merged(&place, base, current, revision, body, merge)?
            }
            _ => (
                body.to_vec(),
                parents(base, current_revision.as_deref()),
                false,
            ),
        };

        self.store(&place, &body, parents)?;

        Ok(Written {
            info: info(&place, &body),
            merged,
        })
    }

    pub(crate) fn remove(&self, project: &str, path: &str) -> Result<(), GuestError> {
        let place = self.place(self.project(project)?, path)?;
        place
            .check_folders()
            .map_err(|error| missing_or_failed(&place, &error))?;

        fs::remove_file(place.file()).map_err(|error| missing_or_failed(&place, &error))?;

        self.shelf
            .forget_revisions(&place.project.id, &place.history_key())
            .map_err(|error| GuestError::new("failed", error.to_string()))
    }

    fn merged(
        &self,
        place: &DocumentPlace,
        base: Option<&str>,
        current: &[u8],
        current_revision: &str,
        incoming: &[u8],
        merge: Merge<'_>,
    ) -> Result<(Vec<u8>, Vec<String>, bool), GuestError> {
        let base_body = base
            .map(|base| self.known_revision(place, base))
            .transpose()?;

        let merged = merge(&place.kind, base_body.as_deref(), current, incoming)?;
        check_size(merged.len())?;

        let incoming_revision = StoredRevision {
            revision: sha256(incoming),
            parents: base.map(str::to_owned).into_iter().collect(),
            body: incoming.to_vec(),
        };
        self.remember(place, &incoming_revision)?;

        let parents = vec![current_revision.to_owned(), incoming_revision.revision];
        Ok((merged, parents, true))
    }

    fn store(
        &self,
        place: &DocumentPlace,
        body: &[u8],
        parents: Vec<String>,
    ) -> Result<(), GuestError> {
        place.prepare_folders().map_err(|error| failed(&error))?;
        replace_file(&place.file(), body).map_err(|error| failed(&error))?;

        let revision = StoredRevision {
            revision: sha256(body),
            parents,
            body: body.to_vec(),
        };

        self.remember(place, &revision)
    }

    fn note_outside_change(&self, place: &DocumentPlace, body: &[u8]) -> Result<(), GuestError> {
        let key = place.history_key();
        let revision = sha256(body);
        let head = self
            .shelf
            .head(&place.project.id, &key)
            .map_err(|error| GuestError::new("failed", error.to_string()))?;

        if head.as_deref() == Some(revision.as_str()) {
            return Ok(());
        }

        let outside = StoredRevision {
            revision,
            parents: head.into_iter().collect(),
            body: body.to_vec(),
        };

        self.remember(place, &outside)
    }

    fn known_revision(&self, place: &DocumentPlace, revision: &str) -> Result<Vec<u8>, GuestError> {
        let stored = self
            .shelf
            .revision(&place.project.id, &place.history_key(), revision)
            .map_err(|error| GuestError::new("failed", error.to_string()))?;

        stored.map(|stored| stored.body).ok_or_else(|| {
            GuestError::new(
                "conflict",
                format!(
                    "{} changed and its base revision {revision} is no longer known; \
                     read it again",
                    place.path.as_str()
                ),
            )
        })
    }

    fn remember(&self, place: &DocumentPlace, revision: &StoredRevision) -> Result<(), GuestError> {
        self.shelf
            .add_revision(&place.project.id, &place.history_key(), revision)
            .map_err(|error| GuestError::new("failed", error.to_string()))
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
            .map_err(|reason| GuestError::new("unknown_project", reason))
    }

    fn place(&self, project: ProjectFolder, path: &str) -> Result<DocumentPlace, GuestError> {
        let path =
            DocumentPath::parse(path).map_err(|reason| GuestError::new("invalid_path", reason))?;

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

fn read(place: &DocumentPlace) -> Result<Option<Vec<u8>>, GuestError> {
    if let Err(error) = place.check_folders() {
        return match error.kind() {
            io::ErrorKind::NotFound => Ok(None),
            _ => Err(failed(&error)),
        };
    }

    match read_plain_file(&place.file(), LARGEST_DOCUMENT) {
        Ok(body) => Ok(Some(body)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(failed(&error)),
    }
}

fn collect(folder: &Path, prefix: &str, depth: usize, found: &mut Vec<String>) -> io::Result<()> {
    if depth >= DEEPEST || found.len() >= MOST_LISTED {
        return Ok(());
    }

    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry.file_type()?;
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };

        if name.starts_with('.') || kind.is_symlink() {
            continue;
        }

        if kind.is_dir() {
            collect(&entry.path(), &relative, depth + 1, found)?;
        } else if kind.is_file() {
            found.push(relative);
        }
    }

    found.sort();
    Ok(())
}

fn info(place: &DocumentPlace, body: &[u8]) -> DocumentInfo {
    DocumentInfo {
        project: place.project.id.clone(),
        path: place.path.as_str().to_owned(),
        kind: place.kind.clone(),
        revision: sha256(body),
        size: u64::try_from(body.len()).unwrap_or(u64::MAX),
    }
}

fn parents(base: Option<&str>, current: Option<&str>) -> Vec<String> {
    current.or(base).map(str::to_owned).into_iter().collect()
}

fn check_size(size: usize) -> Result<(), GuestError> {
    if u64::try_from(size).unwrap_or(u64::MAX) <= LARGEST_DOCUMENT {
        return Ok(());
    }

    Err(GuestError::new(
        "too_large",
        format!("a document holds at most {LARGEST_DOCUMENT} bytes"),
    ))
}

fn not_found(place: &DocumentPlace) -> GuestError {
    GuestError::new(
        "not_found",
        format!("there is no document {}", place.path.as_str()),
    )
}

fn missing_or_failed(place: &DocumentPlace, error: &io::Error) -> GuestError {
    match error.kind() {
        io::ErrorKind::NotFound => not_found(place),
        _ => GuestError::new("failed", error.to_string()),
    }
}

fn failed(error: &io::Error) -> GuestError {
    GuestError::new("failed", error.to_string())
}
