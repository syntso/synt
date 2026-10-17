use std::fs::File;
use std::io;
use std::path::Path;

pub(super) trait NativeFiles {
    fn open_no_follow(path: &Path) -> io::Result<File>;

    fn open_without_waiting(path: &Path) -> io::Result<File>;

    fn create_new_file(path: &Path) -> io::Result<File>;
}
