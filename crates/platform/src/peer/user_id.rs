mod native;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::fmt;
use std::io;

use native::NativeUser;
#[cfg(unix)]
use unix::Unix as Os;
#[cfg(windows)]
use windows::Windows as Os;

#[cfg(unix)]
type Raw = u32;

#[cfg(windows)]
type Raw = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserId(Raw);

impl UserId {
    pub(crate) fn new(raw: Raw) -> Self {
        Self(raw)
    }

    pub(crate) fn current() -> io::Result<Self> {
        Os::current()
    }

    #[cfg(windows)]
    pub(crate) fn of_process(process: u32) -> io::Result<Self> {
        Os::of_process(process)
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
