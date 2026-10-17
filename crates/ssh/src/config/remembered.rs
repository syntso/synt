use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ssh2_config::SshConfig;
use synt_platform::Lock;

static LAST: Mutex<Option<Remembered>> = Mutex::new(None);

pub(super) struct Remembered {
    path: PathBuf,
    text: Vec<u8>,
    parsed: Arc<SshConfig>,
}

impl Remembered {
    pub(super) fn recall(path: &Path, text: &[u8]) -> Option<Arc<SshConfig>> {
        let last = LAST.locked();
        let remembered = last.as_ref()?;

        remembered
            .holds(path, text)
            .then(|| Arc::clone(&remembered.parsed))
    }

    pub(super) fn keep(path: PathBuf, text: Vec<u8>, parsed: Arc<SshConfig>) {
        let remembered = Self { path, text, parsed };

        *LAST.locked() = Some(remembered);
    }

    fn holds(&self, path: &Path, text: &[u8]) -> bool {
        self.path == path && self.text == text
    }
}
