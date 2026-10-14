use super::Visit;
use crate::ExtensionError;
use crate::interface::{GuestError, Level};

const LONGEST_KEY: usize = 256;
const LONGEST_LOG: usize = 4096;

impl Visit {
    pub(super) fn read_item(&self, key: &str) -> Option<Vec<u8>> {
        if !self.manifest().capabilities.storage {
            return None;
        }

        match self.guest.services().shelf.item(self.id(), key) {
            Ok(value) => value,
            Err(failure) => {
                eprintln!("synt-core: extension {}: storage: {failure}", self.id());
                None
            }
        }
    }

    pub(super) fn write_item(&self, key: &str, value: &[u8]) -> Result<(), GuestError> {
        self.allowed(self.manifest().capabilities.storage, "storage = true")?;

        if key.is_empty() || key.len() > LONGEST_KEY {
            return Err(GuestError::new(
                "invalid_key",
                format!("a key holds 1 to {LONGEST_KEY} bytes"),
            ));
        }

        self.guest
            .services()
            .shelf
            .put_item(self.id(), key, value)
            .map_err(|failure| storage_error(&failure))
    }

    pub(super) fn read_settings(&self) -> String {
        let stored = self.guest.services().shelf.settings(self.id());

        match stored {
            Ok(Some(values)) => values.to_string(),
            Ok(None) => "{}".to_owned(),
            Err(failure) => {
                eprintln!("synt-core: extension {}: settings: {failure}", self.id());
                "{}".to_owned()
            }
        }
    }

    pub(super) fn write_log(&mut self, level: Level, message: &str) {
        let shown: String = message
            .chars()
            .take(LONGEST_LOG)
            .map(|character| {
                if character.is_control() {
                    ' '
                } else {
                    character
                }
            })
            .collect();

        if !self.logs.admit(shown.len()) {
            self.note_cut();
            return;
        }

        eprintln!(
            "synt-core: extension {}: {}: {shown}",
            self.id(),
            level.name()
        );
    }

    fn note_cut(&mut self) {
        if self.logs.cut_once() {
            eprintln!(
                "synt-core: extension {}: further log lines of this call are dropped",
                self.id()
            );
        }
    }
}

fn storage_error(failure: &ExtensionError) -> GuestError {
    match failure {
        ExtensionError::Refused(reason) => GuestError::new("storage_full", reason.clone()),
        other => GuestError::new("storage_failed", other.to_string()),
    }
}
