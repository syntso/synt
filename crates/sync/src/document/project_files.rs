use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use synt_platform::{read_plain_file, replace_file};

use super::DocumentPath;
use super::document_path::{EXTENSIONS, MOST_EXTENSION_PARTS, PROJECT_FILE, RESOURCES, RUNBOOKS};
use super::revision::LARGEST_BODY;
use crate::SyncError;

#[derive(Debug, Clone)]
pub(crate) struct ProjectFiles {
    root: PathBuf,
}

impl ProjectFiles {
    pub(crate) fn new(root: &Path) -> Self {
        Self {
            root: root.to_owned(),
        }
    }

    pub(crate) fn paths(&self) -> Result<BTreeSet<DocumentPath>, SyncError> {
        let mut found = BTreeSet::new();

        if is_file(&self.root.join(PROJECT_FILE))? {
            found.insert(DocumentPath::parse(PROJECT_FILE)?);
        }

        for folder in [RESOURCES, RUNBOOKS] {
            found.extend(self.documents_in(folder)?);
        }

        for extension in self.folders_in(EXTENSIONS)? {
            let folder = format!("{EXTENSIONS}/{extension}");
            found.extend(self.documents_below(&folder, MOST_EXTENSION_PARTS)?);
        }

        Ok(found)
    }

    pub(crate) fn read(&self, path: &DocumentPath) -> Result<Option<Vec<u8>>, SyncError> {
        self.check_folders(path)?;

        let limit = u64::try_from(LARGEST_BODY).unwrap_or(u64::MAX);

        match read_plain_file(&path.in_folder(&self.root), limit) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(SyncError::Io(error)),
        }
    }

    pub(crate) fn write(&self, path: &DocumentPath, bytes: &[u8]) -> Result<(), SyncError> {
        for folder in path.folders() {
            self.prepare_folder(&folder)?;
        }

        replace_file(&path.in_folder(&self.root), bytes)?;

        Ok(())
    }

    pub(crate) fn remove(&self, path: &DocumentPath) -> Result<(), SyncError> {
        self.check_folders(path)?;

        match fs::remove_file(path.in_folder(&self.root)) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(SyncError::Io(error)),
            _ => Ok(()),
        }
    }

    fn documents_in(&self, folder: &str) -> Result<Vec<DocumentPath>, SyncError> {
        let names = self.entries(folder, fs::FileType::is_file)?;

        Ok(names
            .into_iter()
            .filter_map(|name| DocumentPath::parse(&format!("{folder}/{name}")).ok())
            .collect())
    }

    fn documents_below(&self, folder: &str, depth: usize) -> Result<Vec<DocumentPath>, SyncError> {
        let mut found = self.documents_in(folder)?;

        if depth > 1 {
            for inner in self.folders_in(folder)? {
                found.extend(self.documents_below(&format!("{folder}/{inner}"), depth - 1)?);
            }
        }

        Ok(found)
    }

    fn folders_in(&self, folder: &str) -> Result<Vec<String>, SyncError> {
        let folders = self.entries(folder, fs::FileType::is_dir)?;

        Ok(folders
            .into_iter()
            .filter(|name| !name.starts_with('.'))
            .collect())
    }

    fn entries(
        &self,
        folder: &str,
        wanted: fn(&fs::FileType) -> bool,
    ) -> Result<Vec<String>, SyncError> {
        let path = self.root.join(folder);

        if !is_folder(&path)? {
            return Ok(Vec::new());
        }

        let mut names = Vec::new();

        for entry in fs::read_dir(&path)? {
            let entry = entry?;

            if wanted(&entry.file_type()?)
                && let Some(name) = entry.file_name().to_str()
            {
                names.push(name.to_owned());
            }
        }

        Ok(names)
    }

    fn check_folders(&self, path: &DocumentPath) -> Result<(), SyncError> {
        for folder in path.folders() {
            let full = self.root.join(&folder);

            if full.symlink_metadata().is_ok() && !is_folder(&full)? {
                return Err(not_a_folder(&full));
            }
        }

        Ok(())
    }

    fn prepare_folder(&self, folder: &str) -> Result<(), SyncError> {
        let path = self.root.join(folder);

        match fs::create_dir(&path) {
            Err(error) if error.kind() != io::ErrorKind::AlreadyExists => {
                return Err(SyncError::Io(error));
            }
            _ => {}
        }

        if is_folder(&path)? {
            return Ok(());
        }

        Err(not_a_folder(&path))
    }
}

fn is_folder(path: &Path) -> Result<bool, SyncError> {
    kind(path, fs::FileType::is_dir)
}

fn is_file(path: &Path) -> Result<bool, SyncError> {
    kind(path, fs::FileType::is_file)
}

fn kind(path: &Path, wanted: fn(&fs::FileType) -> bool) -> Result<bool, SyncError> {
    match path.symlink_metadata() {
        Ok(metadata) => Ok(wanted(&metadata.file_type())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(SyncError::Io(error)),
    }
}

fn not_a_folder(path: &Path) -> SyncError {
    SyncError::Io(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "{} must be a plain folder, not a link or a file",
            path.display()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::ProjectFiles;
    use crate::document::DocumentPath;

    fn path(text: &str) -> DocumentPath {
        DocumentPath::parse(text).unwrap()
    }

    fn names(files: &ProjectFiles) -> Vec<String> {
        files
            .paths()
            .unwrap()
            .iter()
            .map(|path| path.get().to_owned())
            .collect()
    }

    #[test]
    fn only_documents_are_found() {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path();
        fs::create_dir_all(root.join("resources")).unwrap();
        fs::create_dir_all(root.join("extensions/uml/flows")).unwrap();
        fs::create_dir_all(root.join("extensions/uml/.cache")).unwrap();
        fs::create_dir_all(root.join("documents/uml")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join("synt.toml"), "id = 1").unwrap();
        fs::write(root.join("notes.md"), "notes").unwrap();
        fs::write(root.join("resources/api-1.toml"), "a = 1").unwrap();
        fs::write(root.join("resources/.api-1.toml.pending"), "a = 1").unwrap();
        fs::write(root.join("resources/readme.txt"), "text").unwrap();
        fs::write(root.join("extensions/uml/orders.uml"), "@startuml").unwrap();
        fs::write(root.join("extensions/uml/flows/billing.uml"), "@startuml").unwrap();
        fs::write(root.join("extensions/uml/.cache/old.uml"), "@startuml").unwrap();
        fs::write(root.join("documents/uml/orders.uml"), "@startuml").unwrap();
        fs::write(root.join(".git/config"), "x").unwrap();

        let files = ProjectFiles::new(root);

        assert_eq!(
            names(&files),
            vec![
                "extensions/uml/flows/billing.uml",
                "extensions/uml/orders.uml",
                "resources/api-1.toml",
                "synt.toml"
            ]
        );
    }

    #[test]
    fn a_written_document_is_read_back_and_removed() {
        let folder = tempfile::tempdir().unwrap();
        let files = ProjectFiles::new(folder.path());
        let runbook = path("runbooks/restart.toml");

        files.write(&runbook, b"name = \"Restart\"").unwrap();
        assert_eq!(
            files.read(&runbook).unwrap().unwrap(),
            b"name = \"Restart\""
        );

        files.remove(&runbook).unwrap();
        assert_eq!(files.read(&runbook).unwrap(), None);
        files.remove(&runbook).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_folder_is_never_written_through_or_scanned() {
        let folder = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), folder.path().join("resources")).unwrap();
        fs::write(outside.path().join("api-1.toml"), "a = 1").unwrap();

        let files = ProjectFiles::new(folder.path());

        assert!(names(&files).is_empty());
        assert!(files.write(&path("resources/db.toml"), b"a = 1").is_err());
        assert!(files.read(&path("resources/api-1.toml")).is_err());
        assert!(!outside.path().join("db.toml").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_file_is_not_read() {
        let folder = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret"), "secret").unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret"),
            folder.path().join("synt.toml"),
        )
        .unwrap();

        let files = ProjectFiles::new(folder.path());

        assert!(names(&files).is_empty());
        assert!(files.read(&path("synt.toml")).is_err());
    }
}
