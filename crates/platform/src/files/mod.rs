mod limited;
mod owned;
mod plain;
mod presence;
mod replace;

pub use owned::read_owned_file;
pub use plain::read_plain_file;
#[cfg(unix)]
pub(crate) use presence::found;
pub(crate) use presence::remove_if_present;
pub use replace::replace_file;
