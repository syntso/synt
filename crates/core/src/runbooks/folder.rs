mod entry;
mod listing;
mod summary;

pub use entry::RunbookEntry;
pub use listing::RunbookListing;
pub use summary::RunbookSummary;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use synt_platform::{read_plain_file, replace_file};

use super::document::RunbookDocument;
use super::{RunbookError, RunbookName};
use crate::CoreError;
use crate::projects::{Problem, ProjectPlace};

const FOLDER: &str = "runbooks";
const EXTENSION: &str = "toml";
const LARGEST_FILE: u64 = 256 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct RunbookFolder {
    place: ProjectPlace,
}

impl RunbookFolder {
    pub(crate) fn new(place: ProjectPlace) -> Self {
        Self { place }
    }

    pub(crate) fn reference(&self, name: &RunbookName) -> String {
        format!("@{}/{name}", self.place.id)
    }

    pub(crate) fn listing(&self) -> Result<RunbookListing, CoreError> {
        let mut listing = RunbookListing::new(self.place.slug.as_str());

        for name in self.names()? {
            match Self::read_file(&self.path(&name)) {
                Ok(document) => listing.found(RunbookSummary::of(name, &document)),
                Err(error) => listing.unreadable(Problem {
                    file: Self::file_name(&name),
                    error: error.to_string(),
                }),
            }
        }

        Ok(listing)
    }

    pub(crate) fn read(&self, name: &RunbookName) -> Result<RunbookDocument, RunbookError> {
        Self::read_file(&self.path(name))
            .map_err(|error| error.in_project(name, self.place.slug.as_str()))
    }

    pub(crate) fn put(
        &self,
        name: RunbookName,
        document: RunbookDocument,
        admitted: Option<&str>,
    ) -> Result<RunbookEntry, RunbookError> {
        document.check()?;
        self.admit(&name, admitted)?;

        let text = document
            .to_toml()
            .map_err(|source| RunbookError::Unwritable {
                name: name.clone(),
                source,
            })?;

        self.write(&name, &text)
            .map_err(|source| RunbookError::NotSaved {
                name: name.clone(),
                source,
            })?;

        Ok(RunbookEntry::new(name, document))
    }

    fn admit(&self, name: &RunbookName, admitted: Option<&str>) -> Result<(), RunbookError> {
        let actual = self.reference(name);

        if admitted == Some(actual.as_str()) {
            return Ok(());
        }

        Err(RunbookError::Moved {
            name: name.clone(),
            project: self.place.slug.to_string(),
            actual,
        })
    }

    fn write(&self, name: &RunbookName, text: &str) -> io::Result<()> {
        self.prepare()?;

        replace_file(&self.path(name), text.as_bytes())
    }

    fn path(&self, name: &RunbookName) -> PathBuf {
        self.place.path.join(Self::file_name(name))
    }

    fn names(&self) -> Result<Vec<RunbookName>, CoreError> {
        let folder = self.place.path.join(FOLDER);

        let entries = match fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };

        let mut names = Vec::new();

        for entry in entries {
            names.extend(Self::runbook_name(&entry?.path()));
        }

        names.sort();
        Ok(names)
    }

    fn prepare(&self) -> io::Result<()> {
        let folder = self.place.path.join(FOLDER);

        match fs::create_dir(&folder) {
            Err(error) if error.kind() != io::ErrorKind::AlreadyExists => return Err(error),
            _ => {}
        }

        if fs::symlink_metadata(&folder)?.is_dir() {
            return Ok(());
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} must be a plain folder, not a link", folder.display()),
        ))
    }

    fn file_name(name: &RunbookName) -> String {
        format!("{FOLDER}/{name}.{EXTENSION}")
    }

    fn runbook_name(path: &Path) -> Option<RunbookName> {
        if path.extension()? != EXTENSION {
            return None;
        }

        let stem = path.file_stem()?.to_str()?;

        RunbookName::new(stem).ok()
    }

    fn read_file(path: &Path) -> Result<RunbookDocument, RunbookError> {
        let bytes = match read_plain_file(path, LARGEST_FILE) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(RunbookError::NoSuchRunbook);
            }
            Err(error) => return Err(RunbookError::Unreadable(error)),
        };

        let text = String::from_utf8(bytes).map_err(|_| RunbookError::NotUtf8)?;

        RunbookDocument::from_toml(&text)
    }
}
