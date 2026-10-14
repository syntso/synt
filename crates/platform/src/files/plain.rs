use std::io;
use std::path::Path;

use super::limited::read_at_most;
use crate::native;

pub fn read_plain_file(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let file = native::open_no_follow(path)?;

    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a regular file", path.display()),
        ));
    }

    read_at_most(file, limit)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is larger than {limit} bytes", path.display()),
        )
    })
}
