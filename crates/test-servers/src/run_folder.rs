use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::HostKey;

const ON: &str = "1";

#[derive(Debug, Clone)]
pub(crate) struct RunFolder {
    path: PathBuf,
    switch: &'static str,
    start: &'static str,
}

impl RunFolder {
    pub(crate) fn switched_on(
        switch: &'static str,
        start: &'static str,
        folder: &str,
        servers: &str,
    ) -> Option<Self> {
        if env::var(switch).as_deref() != Ok(ON) {
            eprintln!("skipped: set {switch}=1 after {start} to run tests against {servers}");
            return None;
        }

        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(folder);

        Some(Self {
            path,
            switch,
            start,
        })
    }

    pub(crate) fn start(&self) -> &'static str {
        self.start
    }

    pub(crate) fn file(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    pub(crate) fn read(&self, name: &str) -> String {
        let path = self.file(name);

        fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "{}=1 but {} cannot be read ({error}); run {} first",
                self.switch,
                path.display(),
                self.start
            )
        })
    }

    pub(crate) fn bastion_port(&self) -> u16 {
        self.read("bastion_port")
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("{} wrote a bastion port that is not a number", self.start))
    }

    pub(crate) fn host_key(&self, host: &str) -> HostKey {
        let line = self.read(&format!("{host}/host_ed25519_key.pub"));

        HostKey::parse(&line)
            .unwrap_or_else(|| panic!("the {host} host key from {} is damaged", self.start))
    }
}
