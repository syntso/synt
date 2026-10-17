mod entry;

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use synt_platform::{read_plain_file, replace_file};

use crate::SyncError;
use crate::document::{
    DocumentPath, EXTENSIONS, ExtensionDocumentPath, LARGEST_BODY, PROJECT_FILE, RESOURCES,
    RUNBOOKS,
};
use crate::id::ProjectId;
use entry::Entry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFolder {
    id: ProjectId,
    root: PathBuf,
}

impl ProjectFolder {
    pub fn new(id: ProjectId, root: PathBuf) -> Self {
        Self { id, root }
    }

    pub fn id(&self) -> &ProjectId {
        &self.id
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn check(&self) -> Result<(), SyncError> {
        if Entry::at(&self.root)? != Entry::Folder {
            return Err(SyncError::FolderGone(self.root.clone()));
        }

        if self.project_file_id()?.as_ref() == Some(&self.id) {
            return Ok(());
        }

        Err(SyncError::ProjectFileGone {
            folder: self.root.clone(),
            project: self.id.clone(),
        })
    }

    pub(crate) fn paths(&self) -> Result<BTreeSet<DocumentPath>, SyncError> {
        let mut found = BTreeSet::new();

        if Entry::at(&self.root.join(PROJECT_FILE))? == Entry::File {
            found.insert(DocumentPath::parse(PROJECT_FILE)?);
        }

        for folder in [RESOURCES, RUNBOOKS] {
            found.extend(self.documents_in(folder)?);
        }

        for extension in self.folders_in(EXTENSIONS)? {
            let folder = format!("{EXTENSIONS}/{extension}");
            found.extend(self.documents_below(&folder, ExtensionDocumentPath::MOST_PARTS)?);
        }

        Ok(found)
    }

    pub(crate) fn read(&self, path: &DocumentPath) -> Result<Option<Vec<u8>>, SyncError> {
        self.check_folders(path)?;

        let limit = u64::try_from(LARGEST_BODY).unwrap_or(u64::MAX);

        match read_plain_file(&path.in_folder(&self.root), limit) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            read => Ok(Some(read?)),
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
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            removed => Ok(removed?),
        }
    }

    fn project_file_id(&self) -> Result<Option<ProjectId>, SyncError> {
        let Some(bytes) = self.read(&DocumentPath::parse(PROJECT_FILE)?)? else {
            return Ok(None);
        };

        let table = std::str::from_utf8(&bytes)
            .ok()
            .and_then(|text| text.parse::<toml::Table>().ok());

        Ok(table
            .as_ref()
            .and_then(|table| table.get("id"))
            .and_then(toml::Value::as_str)
            .and_then(|id| ProjectId::parse(id).ok()))
    }

    fn documents_in(&self, folder: &str) -> Result<Vec<DocumentPath>, SyncError> {
        let names = self.entries(folder, Entry::File)?;

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
        let folders = self.entries(folder, Entry::Folder)?;

        Ok(folders
            .into_iter()
            .filter(|name| !name.starts_with('.'))
            .collect())
    }

    fn entries(&self, folder: &str, wanted: Entry) -> Result<Vec<String>, SyncError> {
        let path = self.root.join(folder);

        if Entry::at(&path)? != Entry::Folder {
            return Ok(Vec::new());
        }

        let mut names = Vec::new();

        for entry in fs::read_dir(&path)? {
            let entry = entry?;

            if Entry::of(entry.file_type()?) == wanted
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

            if !matches!(Entry::at(&full)?, Entry::Missing | Entry::Folder) {
                return Err(SyncError::NotPlainFolder(full));
            }
        }

        Ok(())
    }

    fn prepare_folder(&self, folder: &str) -> Result<(), SyncError> {
        let path = self.root.join(folder);

        match fs::create_dir(&path) {
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            created => created?,
        }

        if Entry::at(&path)? == Entry::Folder {
            return Ok(());
        }

        Err(SyncError::NotPlainFolder(path))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::ProjectFolder;
    use crate::document::DocumentPath;
    use crate::id::ProjectId;

    fn folder_at(root: &Path) -> ProjectFolder {
        ProjectFolder::new(ProjectId::parse(&"4f".repeat(16)).unwrap(), root.to_owned())
    }

    fn path(text: &str) -> DocumentPath {
        DocumentPath::parse(text).unwrap()
    }

    fn names(files: &ProjectFolder) -> Vec<String> {
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

        let files = folder_at(root);

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
        let files = folder_at(folder.path());
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

        let files = folder_at(folder.path());

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

        let files = folder_at(folder.path());

        assert!(names(&files).is_empty());
        assert!(files.read(&path("synt.toml")).is_err());
    }
}
