use std::fs::{File, OpenOptions, Permissions};
use std::io;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

use nix::libc::{O_NOFOLLOW, O_NONBLOCK};

const SHARED_FILE_MODE: u32 = 0o644;

pub(crate) fn open_no_follow(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK)
        .open(path)
}

pub(crate) fn create_new_file(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(O_NOFOLLOW)
        .mode(SHARED_FILE_MODE)
        .open(path)?;

    file.set_permissions(Permissions::from_mode(SHARED_FILE_MODE))?;

    Ok(file)
}
