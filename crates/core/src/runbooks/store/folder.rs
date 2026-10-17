use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use synt_platform::{read_plain_file, replace_file};

use super::{RunbookListing, RunbookSummary};
use crate::CoreError;
use crate::projects::{Problem, ProjectPlace, check_name};
use crate::runbooks::document::Runbook;

const FOLDER: &str = "runbooks";
const EXTENSION: &str = "toml";
const LARGEST_FILE: u64 = 256 * 1024;

#[derive(Debug, Clone)]
pub(super) struct RunbookFolder {
    place: ProjectPlace,
}

impl RunbookFolder {
    pub(super) fn new(place: ProjectPlace) -> Self {
        Self { place }
    }

    pub(super) fn slug(&self) -> &str {
        self.place.slug.as_str()
    }

    pub(super) fn reference(&self, name: &str) -> String {
        format!("@{}/{name}", self.place.id)
    }

    pub(super) fn admit(&self, name: &str, admitted: Option<&str>) -> Result<(), CoreError> {
        let actual = self.reference(name);

        if admitted == Some(actual.as_str()) {
            return Ok(());
        }

        Err(CoreError::Project(format!(
            "runbook {name} of project {} is now {actual}, which is not what the call was \
             admitted for; run it again",
            self.place.slug
        )))
    }

    pub(super) fn listing(&self) -> Result<RunbookListing, CoreError> {
        let mut listing = RunbookListing {
            project: self.place.slug.to_string(),
            runbooks: Vec::new(),
            problems: Vec::new(),
        };

        for (name, path) in self.files()? {
            match Self::read_file(&path) {
                Ok(runbook) => listing.runbooks.push(RunbookSummary::of(name, &runbook)),
                Err(error) => listing.problems.push(Problem {
                    file: Self::file_name(&name),
                    error,
                }),
            }
        }

        Ok(listing)
    }

    pub(super) fn read(&self, name: &str) -> Result<Runbook, String> {
        Self::read_file(&self.path(name))
    }

    pub(super) fn write(&self, name: &str, text: &str) -> io::Result<()> {
        self.prepare()?;

        replace_file(&self.path(name), text.as_bytes())
    }

    fn path(&self, name: &str) -> PathBuf {
        self.place.path.join(Self::file_name(name))
    }

    fn files(&self) -> Result<Vec<(String, PathBuf)>, CoreError> {
        let folder = self.place.path.join(FOLDER);

        let entries = match fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };

        let mut found = Vec::new();

        for entry in entries {
            let path = entry?.path();

            if let Some(name) = Self::runbook_name(&path) {
                found.push((name, path));
            }
        }

        found.sort();
        Ok(found)
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

    fn file_name(name: &str) -> String {
        format!("{FOLDER}/{name}.{EXTENSION}")
    }

    fn runbook_name(path: &Path) -> Option<String> {
        if path.extension()? != EXTENSION {
            return None;
        }

        let stem = path.file_stem()?.to_str()?;

        check_name(stem).ok().map(|()| stem.to_owned())
    }

    fn read_file(path: &Path) -> Result<Runbook, String> {
        let bytes = match read_plain_file(path, LARGEST_FILE) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err("there is no such runbook".to_owned());
            }
            Err(error) => return Err(error.to_string()),
        };

        let text = String::from_utf8(bytes).map_err(|_| "the file is not UTF-8 text".to_owned())?;

        Runbook::from_toml(&text)
    }
}
