mod native;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::io;
use std::path::PathBuf;

use native::NativeAccount;
#[cfg(unix)]
use unix::Unix as Os;
#[cfg(windows)]
use windows::Windows as Os;

pub fn home_directory() -> io::Result<PathBuf> {
    Os::home_directory()
}

pub fn user_name() -> Option<String> {
    Os::user_name()
}
