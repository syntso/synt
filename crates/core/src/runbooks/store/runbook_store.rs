use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use synt_platform::{read_plain_file, replace_file};

use super::{RunbookEntry, RunbookListing, RunbookSummary};
use crate::CoreError;
use crate::projects::{Problem, ProjectPlace, Projects, check_name};
use crate::runbooks::document::Runbook;

const FOLDER: &str = "runbooks";
const EXTENSION: &str = "toml";
const LARGEST_FILE: u64 = 256 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct RunbookStore {
    projects: Projects,
}

impl RunbookStore {
    pub(crate) fn new(projects: Projects) -> Self {
        Self { projects }
    }

    pub(crate) fn reference(&self, project: &str, name: &str) -> Result<String, CoreError> {
        check_name(name).map_err(CoreError::Runbook)?;
        let place = self.projects.place(project)?;

        Ok(reference(&place, name))
    }

    pub(crate) fn list(&self, project: &str) -> Result<RunbookListing, CoreError> {
        let place = self.projects.place(project)?;
        let mut listing = RunbookListing {
            project: place.slug.clone(),
            runbooks: Vec::new(),
            problems: Vec::new(),
        };

        for (name, path) in files(&place.path)? {
            match read(&path) {
                Ok(runbook) => listing.runbooks.push(RunbookSummary::of(name, &runbook)),
                Err(error) => listing.problems.push(Problem {
                    file: file_name(&name),
                    error,
                }),
            }
        }

        Ok(listing)
    }

    pub(crate) fn get(&self, project: &str, name: &str) -> Result<(String, Runbook), CoreError> {
        check_name(name).map_err(CoreError::Runbook)?;
        let place = self.projects.place(project)?;
        let path = place.path.join(file_name(name));

        let runbook = read(&path).map_err(|error| {
            CoreError::Runbook(format!("runbook {name} of project {}: {error}", place.slug))
        })?;

        Ok((reference(&place, name), runbook))
    }

    pub(crate) fn put(
        &self,
        project: &str,
        name: &str,
        runbook: Runbook,
        admitted: Option<&str>,
    ) -> Result<RunbookEntry, CoreError> {
        check_name(name).map_err(CoreError::Runbook)?;
        runbook.check().map_err(CoreError::Runbook)?;

        let place = self.projects.place(project)?;
        check_admitted(&place, name, admitted)?;

        let text = runbook
            .to_toml()
            .map_err(|error| CoreError::Runbook(format!("cannot write {name}: {error}")))?;

        prepare_folder(&place.path)
            .and_then(|()| replace_file(&place.path.join(file_name(name)), text.as_bytes()))
            .map_err(|error| CoreError::Project(format!("cannot write runbook {name}: {error}")))?;

        Ok(RunbookEntry {
            name: name.to_owned(),
            runbook,
        })
    }
}

fn reference(place: &ProjectPlace, name: &str) -> String {
    format!("@{}/{name}", place.id)
}

fn check_admitted(
    place: &ProjectPlace,
    name: &str,
    admitted: Option<&str>,
) -> Result<(), CoreError> {
    let actual = reference(place, name);

    if admitted == Some(actual.as_str()) {
        return Ok(());
    }

    Err(CoreError::Project(format!(
        "runbook {name} of project {} is now {actual}, which is not what the call was \
         admitted for; run it again",
        place.slug
    )))
}

fn file_name(name: &str) -> String {
    format!("{FOLDER}/{name}.{EXTENSION}")
}

fn read(path: &Path) -> Result<Runbook, String> {
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

fn files(project: &Path) -> Result<Vec<(String, PathBuf)>, CoreError> {
    let folder = project.join(FOLDER);

    let entries = match fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };

    let mut found = Vec::new();

    for entry in entries {
        let path = entry?.path();

        if let Some(name) = runbook_name(&path) {
            found.push((name, path));
        }
    }

    found.sort();
    Ok(found)
}

fn runbook_name(path: &Path) -> Option<String> {
    if path.extension()? != EXTENSION {
        return None;
    }

    let stem = path.file_stem()?.to_str()?;

    check_name(stem).ok().map(|()| stem.to_owned())
}

fn prepare_folder(project: &Path) -> io::Result<()> {
    let folder = project.join(FOLDER);

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
