use std::fs::{self, DirBuilder, File, Permissions};
use std::io;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::Path;

const DIRECTORY_MODE: u32 = 0o700;

pub(crate) fn create_private_directory(path: &Path) -> io::Result<()> {
    DirBuilder::new()
        .recursive(true)
        .mode(DIRECTORY_MODE)
        .create(path)?;

    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is a symbolic link, not a folder", path.display()),
        ));
    }

    let directory = File::open(path)?;
    directory.set_permissions(Permissions::from_mode(DIRECTORY_MODE))
}
