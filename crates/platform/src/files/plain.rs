use std::io;
use std::path::Path;

use super::{NativeFiles, Os, ReadAtMost};

pub fn read_plain_file(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let file = Os::open_no_follow(path)?;

    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a regular file", path.display()),
        ));
    }

    file.read_at_most(limit)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is larger than {limit} bytes", path.display()),
        )
    })
}
