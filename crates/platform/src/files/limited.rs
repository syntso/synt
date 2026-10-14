use std::fs::File;
use std::io::{self, Read};

pub(super) fn read_at_most(file: File, limit: u64) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;

    if bytes.len() as u64 > limit {
        return Ok(None);
    }

    Ok(Some(bytes))
}
