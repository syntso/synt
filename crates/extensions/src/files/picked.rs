use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use synt_platform::{read_plain_file, replace_file};

use super::{FileAccess, FileHandle};
use crate::ExtensionError;
use crate::call::FailureCode;
use crate::interface::GuestError;

const LARGEST_FILE: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Picked {
    handle: FileHandle,
    name: String,
    access: FileAccess,
    path: PathBuf,
}

impl Picked {
    pub(crate) fn pick(path: &str, access: FileAccess) -> Result<Self, ExtensionError> {
        let path = PathBuf::from(path);

        Self::check_pickable(&path, access)?;

        Ok(Self::stored(FileHandle::fresh()?, path, access))
    }

    pub(crate) fn stored(handle: FileHandle, path: PathBuf, access: FileAccess) -> Self {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        Self {
            handle,
            name,
            access,
            path,
        }
    }

    pub(crate) fn handle(&self) -> &FileHandle {
        &self.handle
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn access(&self) -> FileAccess {
        self.access
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn read(&self) -> Result<Vec<u8>, GuestError> {
        read_plain_file(&self.path, LARGEST_FILE).map_err(|error| self.failed(&error))
    }

    pub(crate) fn write(&self, body: &[u8]) -> Result<(), GuestError> {
        if u64::try_from(body.len()).unwrap_or(u64::MAX) > LARGEST_FILE {
            return Err(GuestError::new(
                FailureCode::TooLarge,
                format!("a picked file holds at most {LARGEST_FILE} bytes"),
            ));
        }

        if fs::symlink_metadata(&self.path).is_ok_and(|found| !found.is_file()) {
            return Err(GuestError::failed(format!(
                "{} is no longer a regular file",
                self.name
            )));
        }

        replace_file(&self.path, body).map_err(|error| self.failed(&error))
    }

    fn failed(&self, error: &io::Error) -> GuestError {
        GuestError::failed(format!("{}: {error}", self.name))
    }

    fn check_pickable(path: &Path, access: FileAccess) -> Result<(), ExtensionError> {
        if !path.is_absolute() {
            return Err(ExtensionError::PickRelative);
        }

        match fs::symlink_metadata(path) {
            Ok(found) if found.is_file() => Ok(()),
            Ok(_) => Err(ExtensionError::PickNotFile(path.to_owned())),
            Err(error) if error.kind() == io::ErrorKind::NotFound && access.writes() => {
                Self::check_parent_folder(path)
            }
            Err(source) => Err(ExtensionError::PickUnreadable {
                path: path.to_owned(),
                source,
            }),
        }
    }

    fn check_parent_folder(path: &Path) -> Result<(), ExtensionError> {
        let has_folder = path.parent().is_some_and(Path::is_dir);

        if !has_folder {
            return Err(ExtensionError::PickFolderMissing(path.to_owned()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::Picked;
    use crate::files::{FileAccess, FileHandle};

    fn pick(path: &Path, access: FileAccess) -> Result<Picked, crate::ExtensionError> {
        Picked::pick(&path.to_string_lossy(), access)
    }

    #[test]
    fn a_file_to_read_must_exist_and_one_to_write_needs_its_folder() {
        let folder = tempfile::tempdir().unwrap();
        let present = folder.path().join("present.txt");
        fs::write(&present, b"hello").unwrap();
        let absent = folder.path().join("absent.txt");

        assert!(pick(&present, FileAccess::Read).is_ok());
        assert!(pick(&absent, FileAccess::Read).is_err());
        assert!(pick(&absent, FileAccess::ReadWrite).is_ok());
        assert!(pick(folder.path(), FileAccess::Read).is_err());
        assert!(pick(Path::new("relative.txt"), FileAccess::Read).is_err());
    }

    #[test]
    fn a_pick_is_named_after_its_file_and_gets_a_fresh_handle() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("notes.txt");
        fs::write(&path, b"x").unwrap();

        let first = pick(&path, FileAccess::Read).unwrap();
        let second = pick(&path, FileAccess::Read).unwrap();

        assert_eq!(first.name(), "notes.txt");
        assert!(first.handle().to_string().starts_with("file-"));
        assert_ne!(first.handle(), second.handle());
    }

    #[test]
    fn writes_replace_the_picked_file() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("notes.txt");
        let handle = FileHandle::from("h".to_owned());
        let picked = Picked::stored(handle, path, FileAccess::ReadWrite);

        picked.write(b"first").unwrap();
        picked.write(b"second").unwrap();

        assert_eq!(picked.read().unwrap(), b"second");
        assert_eq!(picked.name(), "notes.txt");
    }
}
