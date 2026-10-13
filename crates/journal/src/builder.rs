use std::path::Path;
use std::sync::Arc;

use crate::anchor::KeyStore;
use crate::chain::JournalId;
use crate::error::JournalError;
use crate::journal::Journal;
use crate::model::{Clock, SystemClock};

pub struct JournalBuilder {
    keys: Arc<dyn KeyStore>,
    clock: Arc<dyn Clock>,
}

impl JournalBuilder {
    pub fn new(keys: Arc<dyn KeyStore>) -> Self {
        Self {
            keys,
            clock: Arc::new(SystemClock),
        }
    }

    #[must_use]
    pub fn clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    pub fn create(self, path: impl AsRef<Path>) -> Result<Journal, JournalError> {
        Journal::create_with(path.as_ref(), self.keys, self.clock)
    }

    pub fn open(self, path: impl AsRef<Path>, id: JournalId) -> Result<Journal, JournalError> {
        Journal::open_with(path.as_ref(), id, self.keys, self.clock)
    }
}
