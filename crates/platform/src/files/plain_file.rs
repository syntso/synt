use std::io::{self, Read};
use std::path::Path;

use crate::native;

pub fn read_plain_file(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let file = native::open_no_follow(path)?;

    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a regular file", path.display()),
        ));
    }

    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;

    if bytes.len() as u64 > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is larger than {limit} bytes", path.display()),
        ));
    }

    Ok(bytes)
}
