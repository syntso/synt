use std::io;
use std::path::PathBuf;

pub(super) trait NativeAccount {
    fn home_directory() -> io::Result<PathBuf>;

    fn user_name() -> Option<String>;
}
