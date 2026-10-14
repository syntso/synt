use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use tempfile::TempDir;

use super::HostKey;

const KNOWN_HOSTS: &str = "known_hosts";

#[derive(Debug)]
pub struct SshHome {
    directory: TempDir,
}

impl SshHome {
    pub fn empty() -> Self {
        let directory = tempfile::tempdir()
            .unwrap_or_else(|error| panic!("the system offers no temporary folder: {error}"));

        Self { directory }
    }

    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    #[must_use]
    pub fn trusting(self, host: &str, key: &HostKey) -> Self {
        self.trust(host, key);
        self
    }

    pub fn trust(&self, host: &str, key: &HostKey) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.path().join(KNOWN_HOSTS))
            .unwrap_or_else(|error| Self::unwritable(&error));

        writeln!(file, "{}", key.known_hosts_line(host))
            .unwrap_or_else(|error| Self::unwritable(&error));
    }

    pub fn forget_all(&self) {
        fs::write(self.path().join(KNOWN_HOSTS), "")
            .unwrap_or_else(|error| Self::unwritable(&error));
    }

    fn unwritable(error: &io::Error) -> ! {
        panic!("the temporary folder is not writable: {error}")
    }
}
