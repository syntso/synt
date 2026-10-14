mod limited;
mod owned_file;
mod plain_file;
mod presence;
mod replace;

pub use owned_file::read_owned_file;
pub use plain_file::read_plain_file;
#[cfg(unix)]
pub(crate) use presence::found;
pub(crate) use presence::remove_if_present;
pub use replace::replace_file;
