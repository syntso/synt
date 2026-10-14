use std::io;
use std::path::Path;

use synt_platform::read_plain_file;

const LARGEST_FILE: u64 = 64 * 1024;

pub(crate) struct Text(String);

impl Text {
    pub(crate) fn read(path: &Path) -> io::Result<Self> {
        let bytes = read_plain_file(path, LARGEST_FILE)?;

        let text = String::from_utf8(bytes).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} is not UTF-8 text", path.display()),
            )
        })?;

        Ok(Self(text))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn into_string(self) -> String {
        self.0
    }
}
