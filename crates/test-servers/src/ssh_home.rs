use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use tempfile::TempDir;

use crate::HostKey;

const KNOWN_HOSTS: &str = "known_hosts";

#[derive(Debug)]
pub struct SshHome {
    directory: TempDir,
}

impl SshHome {
    pub fn empty() -> Self {
        let directory = tempfile::tempdir().expect("the system offers a temporary folder");

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
            .expect("the temporary folder is writable");

        writeln!(file, "{}", key.known_hosts_line(host)).expect("the temporary folder is writable");
    }

    pub fn forget_all(&self) {
        fs::write(self.path().join(KNOWN_HOSTS), "").expect("the temporary folder is writable");
    }
}
