use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{DocumentPath, ProjectFolder};

const FOLDER: &str = "extensions";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DocumentPlace {
    pub(crate) project: ProjectFolder,
    pub(crate) path: DocumentPath,
    pub(crate) kind: String,
    extension: String,
}

impl DocumentPlace {
    pub(crate) fn new(
        project: ProjectFolder,
        extension: &str,
        path: DocumentPath,
        kind: String,
    ) -> Self {
        Self {
            project,
            path,
            kind,
            extension: extension.to_owned(),
        }
    }

    pub(crate) fn root(project: &ProjectFolder, extension: &str) -> PathBuf {
        project.path.join(FOLDER).join(extension)
    }

    pub(crate) fn file(&self) -> PathBuf {
        self.path.segments().fold(
            Self::root(&self.project, &self.extension),
            |path, segment| path.join(segment),
        )
    }

    pub(crate) fn history_key(&self) -> String {
        format!("{FOLDER}/{}/{}", self.extension, self.path.as_str())
    }

    pub(crate) fn check_folders(&self) -> io::Result<()> {
        self.folders()
            .iter()
            .try_for_each(|folder| plain_folder(folder, false))
    }

    pub(crate) fn prepare_folders(&self) -> io::Result<()> {
        self.folders()
            .iter()
            .try_for_each(|folder| plain_folder(folder, true))
    }

    fn folders(&self) -> Vec<PathBuf> {
        let mut current = Self::root(&self.project, &self.extension);
        let mut folders = vec![self.project.path.join(FOLDER), current.clone()];

        let segments: Vec<&str> = self.path.segments().collect();
        let (_, inner) = segments.split_last().unwrap_or((&"", &[]));

        for segment in inner {
            current = current.join(segment);
            folders.push(current.clone());
        }

        folders
    }
}

fn plain_folder(folder: &Path, create: bool) -> io::Result<()> {
    match fs::symlink_metadata(folder) {
        Ok(found) if found.is_dir() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} must be a plain folder, not a link or a file",
                folder.display()
            ),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound && create => fs::create_dir(folder),
        Err(error) => Err(error),
    }
}
