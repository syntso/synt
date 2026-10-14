use std::fs::{File, Metadata, OpenOptions, Permissions};
use std::io;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

use nix::libc::{O_NOFOLLOW, O_NONBLOCK};
use nix::unistd;

use super::{NativeFiles, Ownership};

const SHARED_FILE_MODE: u32 = 0o644;
const GROUP_OR_OTHER_WRITE: u32 = 0o022;

pub(super) struct Unix;

impl NativeFiles for Unix {
    fn open_no_follow(path: &Path) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .custom_flags(O_NOFOLLOW | O_NONBLOCK)
            .open(path)
    }

    fn open_without_waiting(path: &Path) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .custom_flags(O_NONBLOCK)
            .open(path)
    }

    fn create_new_file(path: &Path) -> io::Result<File> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(O_NOFOLLOW)
            .mode(SHARED_FILE_MODE)
            .open(path)?;

        file.set_permissions(Permissions::from_mode(SHARED_FILE_MODE))?;

        Ok(file)
    }
}

impl Ownership for Metadata {
    fn owned_by_current_user(&self) -> bool {
        self.uid() == unistd::geteuid().as_raw()
    }

    fn writable_by_others(&self) -> bool {
        self.mode() & GROUP_OR_OTHER_WRITE != 0
    }
}
