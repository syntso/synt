use std::fs::Metadata;
use std::io::{self, Read};
use std::path::Path;

use crate::native;

pub fn read_owned_file(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let file = native::open_without_waiting(path)?;
    let metadata = file.metadata()?;

    refuse_unsafe(&metadata)?;

    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;

    if bytes.len() as u64 > limit {
        return Err(refused(&format!("it is larger than {limit} bytes")));
    }

    Ok(bytes)
}

fn refuse_unsafe(metadata: &Metadata) -> io::Result<()> {
    if !metadata.is_file() {
        return Err(refused("it is not a regular file"));
    }

    if !native::owned_by_current_user(metadata) {
        return Err(refused("it belongs to another user"));
    }

    if native::writable_by_others(metadata) {
        return Err(refused("other users can change it"));
    }

    Ok(())
}

fn refused(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::read_owned_file;

    #[test]
    fn a_small_own_file_is_read() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("key");
        fs::write(&path, b"secret").unwrap();

        assert_eq!(read_owned_file(&path, 16).unwrap(), b"secret");
    }

    #[test]
    fn a_large_file_is_refused() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("key");
        fs::write(&path, [0; 32]).unwrap();

        let error = read_owned_file(&path, 16).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn a_folder_is_refused() {
        let folder = tempfile::tempdir().unwrap();

        assert!(read_owned_file(folder.path(), 16).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_file_others_can_change_is_refused() {
        use std::os::unix::fs::PermissionsExt;

        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("key");
        fs::write(&path, b"secret").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();

        let error = read_owned_file(&path, 16).unwrap_err();

        assert_eq!(error.to_string(), "other users can change it");
    }

    #[cfg(unix)]
    #[test]
    fn a_pipe_is_refused_without_waiting() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("key");
        let made = std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap();
        assert!(made.success());

        let error = read_owned_file(&path, 16).unwrap_err();

        assert_eq!(error.to_string(), "it is not a regular file");
    }
}
