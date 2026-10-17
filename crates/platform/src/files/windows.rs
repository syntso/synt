use std::fs::{File, Metadata, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;

use super::{NativeFiles, Ownership};

const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

pub(super) struct Windows;

impl NativeFiles for Windows {
    fn open_no_follow(path: &Path) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
    }

    fn open_without_waiting(path: &Path) -> io::Result<File> {
        File::open(path)
    }

    fn create_new_file(path: &Path) -> io::Result<File> {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
    }
}

impl Ownership for Metadata {
    fn owned_by_current_user(&self) -> bool {
        true
    }

    fn writable_by_others(&self) -> bool {
        false
    }
}
