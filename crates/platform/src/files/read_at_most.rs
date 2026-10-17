use std::fs::File;
use std::io::{self, Read};

pub(super) trait ReadAtMost {
    fn read_at_most(self, limit: u64) -> io::Result<Option<Vec<u8>>>;
}

impl ReadAtMost for File {
    fn read_at_most(self, limit: u64) -> io::Result<Option<Vec<u8>>> {
        let mut bytes = Vec::new();
        self.take(limit + 1).read_to_end(&mut bytes)?;

        if bytes.len() as u64 > limit {
            return Ok(None);
        }

        Ok(Some(bytes))
    }
}
