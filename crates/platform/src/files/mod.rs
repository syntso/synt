mod owned_file;
mod plain_file;
mod replace;

pub use owned_file::read_owned_file;
pub use plain_file::read_plain_file;
pub use replace::replace_file;
