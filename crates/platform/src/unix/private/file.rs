use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const FILE_MODE: u32 = 0o600;
const PENDING_SUFFIX: &str = ".pending";

pub(crate) fn replace_private_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let pending = pending(path);
    remove_stale(&pending)?;

    write_new(&pending, bytes)?;
    fs::rename(&pending, path)?;

    sync_directory(path)
}

fn pending(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(PENDING_SUFFIX);

    PathBuf::from(name)
}

fn remove_stale(pending: &Path) -> io::Result<()> {
    match fs::remove_file(pending) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(FILE_MODE)
        .open(path)?;

    file.set_permissions(Permissions::from_mode(FILE_MODE))?;
    file.write_all(bytes)?;

    file.sync_all()
}

fn sync_directory(path: &Path) -> io::Result<()> {
    let Some(directory) = path.parent() else {
        return Ok(());
    };

    File::open(directory)?.sync_all()
}
