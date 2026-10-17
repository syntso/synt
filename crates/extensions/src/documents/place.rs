use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use synt_platform::{read_plain_file, replace_file};
use synt_sync::ProjectFolder;

use super::DocumentInfo;
use super::path::DocumentPath;
use crate::call::FailureCode;
use crate::interface::GuestError;

const FOLDER: &str = "extensions";

pub(super) const LARGEST_DOCUMENT: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DocumentPlace {
    project: ProjectFolder,
    extension: String,
    path: DocumentPath,
    kind: String,
}

impl DocumentPlace {
    pub(super) fn new(
        project: ProjectFolder,
        extension: &str,
        path: DocumentPath,
        kind: String,
    ) -> Self {
        Self {
            project,
            extension: extension.to_owned(),
            path,
            kind,
        }
    }

    pub(super) fn root(project: &ProjectFolder, extension: &str) -> PathBuf {
        project.root().join(FOLDER).join(extension)
    }

    pub(super) fn project_id(&self) -> &str {
        self.project.id().get()
    }

    pub(super) fn path(&self) -> &str {
        self.path.as_str()
    }

    pub(super) fn kind(&self) -> &str {
        &self.kind
    }

    pub(super) fn history_key(&self) -> String {
        format!("{FOLDER}/{}/{}", self.extension, self.path.as_str())
    }

    pub(super) fn info(&self, body: &[u8]) -> DocumentInfo {
        DocumentInfo::new(
            self.project.id().get().to_owned(),
            self.path.as_str().to_owned(),
            self.kind.clone(),
            body,
        )
    }

    pub(super) fn read(&self) -> Result<Option<Vec<u8>>, GuestError> {
        match self.check_folders() {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            checked => checked?,
        }

        match read_plain_file(&self.file(), LARGEST_DOCUMENT) {
            Ok(body) => Ok(Some(body)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub(super) fn write(&self, body: &[u8]) -> Result<(), GuestError> {
        self.prepare_folders()?;
        replace_file(&self.file(), body)?;

        Ok(())
    }

    pub(super) fn remove(&self) -> Result<(), GuestError> {
        self.check_folders()
            .and_then(|()| fs::remove_file(self.file()))
            .map_err(|error| match error.kind() {
                io::ErrorKind::NotFound => self.not_found(),
                _ => error.into(),
            })
    }

    pub(super) fn not_found(&self) -> GuestError {
        GuestError::new(
            FailureCode::NotFound,
            format!("there is no document {}", self.path.as_str()),
        )
    }

    fn file(&self) -> PathBuf {
        self.path.segments().fold(
            Self::root(&self.project, &self.extension),
            |path, segment| path.join(segment),
        )
    }

    fn check_folders(&self) -> io::Result<()> {
        self.folders()
            .iter()
            .try_for_each(|folder| Self::check_plain(folder))
    }

    fn prepare_folders(&self) -> io::Result<()> {
        self.folders()
            .iter()
            .try_for_each(|folder| Self::create_plain(folder))
    }

    fn folders(&self) -> Vec<PathBuf> {
        let mut current = Self::root(&self.project, &self.extension);
        let mut folders = vec![self.project.root().join(FOLDER), current.clone()];

        let segments: Vec<&str> = self.path.segments().collect();
        let (_, inner) = segments.split_last().unwrap_or((&"", &[]));

        for segment in inner {
            current = current.join(segment);
            folders.push(current.clone());
        }

        folders
    }

    fn check_plain(folder: &Path) -> io::Result<()> {
        if fs::symlink_metadata(folder)?.is_dir() {
            return Ok(());
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} must be a plain folder, not a link or a file",
                folder.display()
            ),
        ))
    }

    fn create_plain(folder: &Path) -> io::Result<()> {
        match Self::check_plain(folder) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(folder),
            checked => checked,
        }
    }
}
