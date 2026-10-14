use std::fs;
use std::io;
use std::path::Path;

use data_encoding::HEXLOWER;
use serde::Serialize;
use synt_platform::{read_plain_file, replace_file};

use crate::ExtensionError;
use crate::interface::GuestError;
use crate::manifest::FileAccess;

const LARGEST_FILE: u64 = 16 * 1024 * 1024;
const HANDLE_BYTES: usize = 12;
const HANDLE_PREFIX: &str = "file-";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Picked {
    pub(crate) handle: String,
    pub(crate) name: String,
    pub(crate) access: FileAccess,

    #[serde(skip)]
    pub(crate) path: String,
}

impl Picked {
    pub(crate) fn pick(path: &str, access: FileAccess) -> Result<Self, ExtensionError> {
        check_pickable(Path::new(path), access).map_err(ExtensionError::Refused)?;

        Ok(Self::stored(new_handle()?, path.to_owned(), access))
    }

    pub(crate) fn stored(handle: String, path: String, access: FileAccess) -> Self {
        let name = Path::new(&path)
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

    pub(crate) fn read(&self) -> Result<Vec<u8>, GuestError> {
        read_plain_file(Path::new(&self.path), LARGEST_FILE).map_err(|error| self.failed(&error))
    }

    pub(crate) fn write(&self, body: &[u8]) -> Result<(), GuestError> {
        if u64::try_from(body.len()).unwrap_or(u64::MAX) > LARGEST_FILE {
            return Err(GuestError::new(
                "too_large",
                format!("a picked file holds at most {LARGEST_FILE} bytes"),
            ));
        }

        let path = Path::new(&self.path);
        if fs::symlink_metadata(path).is_ok_and(|found| !found.is_file()) {
            return Err(GuestError::failed(format!(
                "{} is no longer a regular file",
                self.name
            )));
        }

        replace_file(path, body).map_err(|error| self.failed(&error))
    }

    fn failed(&self, error: &io::Error) -> GuestError {
        GuestError::failed(format!("{}: {error}", self.name))
    }
}

fn check_pickable(path: &Path, access: FileAccess) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("a picked file is named by its absolute path".to_owned());
    }

    match fs::symlink_metadata(path) {
        Ok(found) if found.is_file() => Ok(()),
        Ok(_) => Err(format!("{} is not a regular file", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound && access.writes() => {
            check_parent_folder(path)
        }
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn check_parent_folder(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no folder", path.display()))?;

    if parent.is_dir() {
        return Ok(());
    }

    Err(format!("the folder of {} does not exist", path.display()))
}

fn new_handle() -> Result<String, ExtensionError> {
    let mut bytes = [0_u8; HANDLE_BYTES];

    getrandom::fill(&mut bytes)
        .map_err(|error| ExtensionError::Refused(format!("system random source: {error}")))?;

    Ok(format!("{HANDLE_PREFIX}{}", HEXLOWER.encode(&bytes)))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::Picked;
    use crate::manifest::FileAccess;

    fn picked(path: &Path, access: FileAccess) -> Picked {
        Picked::stored("h".to_owned(), path.to_string_lossy().into_owned(), access)
    }

    #[test]
    fn a_file_to_read_must_exist_and_one_to_write_needs_its_folder() {
        let folder = tempfile::tempdir().unwrap();
        let present = folder.path().join("present.txt");
        fs::write(&present, b"hello").unwrap();
        let absent = folder.path().join("absent.txt");
        let pick = |path: &Path, access| Picked::pick(&path.to_string_lossy(), access);

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

        let first = Picked::pick(&path.to_string_lossy(), FileAccess::Read).unwrap();
        let second = Picked::pick(&path.to_string_lossy(), FileAccess::Read).unwrap();

        assert_eq!(first.name, "notes.txt");
        assert!(first.handle.starts_with("file-"));
        assert_ne!(first.handle, second.handle);
    }

    #[test]
    fn writes_replace_the_picked_file() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("notes.txt");
        let picked = picked(&path, FileAccess::ReadWrite);

        picked.write(b"first").unwrap();
        picked.write(b"second").unwrap();

        assert_eq!(picked.read().unwrap(), b"second");
        assert_eq!(picked.name, "notes.txt");
    }
}
