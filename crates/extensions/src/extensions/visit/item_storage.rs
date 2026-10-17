use std::sync::Arc;

use crate::ExtensionError;
use crate::call::FailureCode;
use crate::extension::Guest;
use crate::interface::GuestError;

const LONGEST_KEY: usize = 256;
const NO_SETTINGS: &str = "{}";

pub(super) struct ItemStorage {
    guest: Arc<Guest>,
}

impl ItemStorage {
    pub(super) fn new(guest: Arc<Guest>) -> Self {
        Self { guest }
    }

    pub(super) fn get(&self, key: &str) -> Option<Vec<u8>> {
        if !self.guest.manifest().capabilities().storage() {
            return None;
        }

        match self
            .guest
            .services()
            .shelf()
            .items()
            .get(self.guest.id(), key)
        {
            Ok(value) => value,
            Err(failure) => {
                self.note("storage", &failure);
                None
            }
        }
    }

    pub(super) fn set(&self, key: &str, value: &[u8]) -> Result<(), GuestError> {
        let storage = self.guest.manifest().capabilities().storage();
        self.guest.require(storage, "storage = true")?;

        if key.is_empty() || key.len() > LONGEST_KEY {
            return Err(GuestError::new(
                FailureCode::InvalidKey,
                format!("a key holds 1 to {LONGEST_KEY} bytes"),
            ));
        }

        self.guest
            .services()
            .shelf()
            .items()
            .put(self.guest.id(), key, value)
            .map_err(|failure| Self::refusal(&failure))
    }

    pub(super) fn settings(&self) -> String {
        let stored = self
            .guest
            .services()
            .shelf()
            .settings()
            .get(self.guest.id());

        match stored {
            Ok(Some(values)) => values.to_string(),
            Ok(None) => NO_SETTINGS.to_owned(),
            Err(failure) => {
                self.note("settings", &failure);
                NO_SETTINGS.to_owned()
            }
        }
    }

    fn note(&self, what: &str, failure: &ExtensionError) {
        eprintln!(
            "synt-core: extension {}: {what}: {failure}",
            self.guest.id()
        );
    }

    fn refusal(failure: &ExtensionError) -> GuestError {
        let code = match failure {
            ExtensionError::StorageFull { .. } => FailureCode::StorageFull,
            _ => FailureCode::StorageFailed,
        };

        GuestError::new(code, failure.to_string())
    }
}
