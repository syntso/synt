mod found;
mod native;
mod owned;
mod ownership;
mod plain;
mod read_at_most;
mod replace;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

pub use found::Found;
use native::NativeFiles;
pub use owned::read_owned_file;
pub(crate) use ownership::Ownership;
pub use plain::read_plain_file;
use read_at_most::ReadAtMost;
pub use replace::replace_file;
#[cfg(unix)]
use unix::Unix as Os;
#[cfg(windows)]
use windows::Windows as Os;
