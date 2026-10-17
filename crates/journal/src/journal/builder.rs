use std::path::Path;
use std::sync::Arc;

use super::{Journal, JournalId};
use crate::clock::{Clock, SystemClock};
use crate::error::JournalError;
use crate::key_store::KeyStore;

pub struct JournalBuilder {
    keys: Arc<dyn KeyStore>,
    clock: Arc<dyn Clock>,
    id: Option<JournalId>,
}

impl JournalBuilder {
    pub fn new(keys: Arc<dyn KeyStore>) -> Self {
        Self {
            keys,
            clock: Arc::new(SystemClock),
            id: None,
        }
    }

    #[must_use]
    pub fn clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    #[must_use]
    pub fn id(mut self, id: JournalId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn create(self, path: impl AsRef<Path>) -> Result<Journal, JournalError> {
        let id = match self.id {
            Some(id) => id,
            None => JournalId::generate()?,
        };

        Journal::create_with(path.as_ref(), id, self.keys, self.clock)
    }

    pub fn open(self, path: impl AsRef<Path>, id: JournalId) -> Result<Journal, JournalError> {
        Journal::open_with(path.as_ref(), id, self.keys, self.clock)
    }
}
