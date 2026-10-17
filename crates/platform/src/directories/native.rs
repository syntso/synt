use std::io;
use std::path::Path;

pub(super) trait NativeDirectories {
    fn create_private_directory(path: &Path) -> io::Result<()>;
}
