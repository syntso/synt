use std::io;
use std::path::PathBuf;

use crate::native;

pub fn home_directory() -> io::Result<PathBuf> {
    native::home_directory()
}

pub fn user_name() -> Option<String> {
    native::user_name()
}
