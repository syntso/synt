use std::fs;
use std::io;
use std::path::Path;

use synt_platform::{read_plain_file, replace_file};

use super::Picked;
use crate::interface::GuestError;
use crate::manifest::FileAccess;

const LARGEST_FILE: u64 = 16 * 1024 * 1024;

pub(crate) struct FileGate;

impl FileGate {
    pub(crate) fn check_pick(path: &Path, access: FileAccess) -> Result<(), String> {
        if !path.is_absolute() {
            return Err("a picked file is named by its absolute path".to_owned());
        }

        match fs::symlink_metadata(path) {
            Ok(found) if found.is_file() => Ok(()),
            Ok(_) => Err(format!("{} is not a regular file", path.display())),
            Err(error) if error.kind() == io::ErrorKind::NotFound && access.writes() => {
                parent_folder(path)
            }
            Err(error) => Err(format!("{}: {error}", path.display())),
        }
    }

    pub(crate) fn read(picked: &Picked) -> Result<Vec<u8>, GuestError> {
        read_plain_file(Path::new(&picked.path), LARGEST_FILE)
            .map_err(|error| failed(picked, &error))
    }

    pub(crate) fn write(picked: &Picked, body: &[u8]) -> Result<(), GuestError> {
        if u64::try_from(body.len()).unwrap_or(u64::MAX) > LARGEST_FILE {
            return Err(GuestError::new(
                "too_large",
                format!("a picked file holds at most {LARGEST_FILE} bytes"),
            ));
        }

        let path = Path::new(&picked.path);
        if fs::symlink_metadata(path).is_ok_and(|found| !found.is_file()) {
            return Err(GuestError::new(
                "failed",
                format!("{} is no longer a regular file", picked.name),
            ));
        }

        replace_file(path, body).map_err(|error| failed(picked, &error))
    }
}

fn parent_folder(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no folder", path.display()))?;

    if parent.is_dir() {
        return Ok(());
    }

    Err(format!("the folder of {} does not exist", path.display()))
}

fn failed(picked: &Picked, error: &io::Error) -> GuestError {
    GuestError::new("failed", format!("{}: {error}", picked.name))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::FileGate;
    use crate::files::Picked;
    use crate::manifest::FileAccess;

    fn picked(path: &std::path::Path, access: FileAccess) -> Picked {
        let path = path.to_string_lossy().into_owned();

        Picked {
            handle: "h".to_owned(),
            name: Picked::name_of(&path),
            access,
            path,
        }
    }

    #[test]
    fn a_file_to_read_must_exist_and_one_to_write_needs_its_folder() {
        let folder = tempfile::tempdir().unwrap();
        let present = folder.path().join("present.txt");
        fs::write(&present, b"hello").unwrap();
        let absent = folder.path().join("absent.txt");

        assert!(FileGate::check_pick(&present, FileAccess::Read).is_ok());
        assert!(FileGate::check_pick(&absent, FileAccess::Read).is_err());
        assert!(FileGate::check_pick(&absent, FileAccess::ReadWrite).is_ok());
        assert!(FileGate::check_pick(folder.path(), FileAccess::Read).is_err());
        assert!(
            FileGate::check_pick(std::path::Path::new("relative.txt"), FileAccess::Read).is_err()
        );
    }

    #[test]
    fn writes_replace_the_picked_file() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("notes.txt");
        let picked = picked(&path, FileAccess::ReadWrite);

        FileGate::write(&picked, b"first").unwrap();
        FileGate::write(&picked, b"second").unwrap();

        assert_eq!(FileGate::read(&picked).unwrap(), b"second");
        assert_eq!(picked.name, "notes.txt");
    }
}
