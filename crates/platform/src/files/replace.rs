use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::native;

const PENDING_SUFFIX: &str = ".pending";

pub fn replace_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let pending = pending(path)?;
    remove_stale(&pending)?;

    let mut file = native::create_new_file(&pending)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);

    fs::rename(&pending, path)
}

fn pending(path: &Path) -> io::Result<PathBuf> {
    let (Some(folder), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} does not name a file", path.display()),
        ));
    };

    let mut pending = OsString::from(".");
    pending.push(name);
    pending.push(PENDING_SUFFIX);

    Ok(folder.join(pending))
}

fn remove_stale(pending: &Path) -> io::Result<()> {
    match fs::remove_file(pending) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}
