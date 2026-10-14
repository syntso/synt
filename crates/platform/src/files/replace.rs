use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::{Found, NativeFiles, Os};

const PENDING_SUFFIX: &str = ".pending";

pub fn replace_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let pending = pending(path)?;
    fs::remove_file(&pending).found()?;

    let mut file = Os::create_new_file(&pending)?;
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
