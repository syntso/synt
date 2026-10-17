use std::sync::Arc;

use crate::call::FailureCode;
use crate::extension::Guest;
use crate::files::{FileAccess, Picked};
use crate::interface::GuestError;

pub(super) struct PickedFiles {
    guest: Arc<Guest>,
}

impl PickedFiles {
    pub(super) fn new(guest: Arc<Guest>) -> Self {
        Self { guest }
    }

    pub(super) fn chosen(&self) -> Vec<Picked> {
        if self.guest.manifest().capabilities().files().is_none() {
            return Vec::new();
        }

        match self.guest.services().shelf().picks().all(self.guest.id()) {
            Ok(picks) => picks,
            Err(failure) => {
                eprintln!(
                    "synt-core: extension {}: picked files: {failure}",
                    self.guest.id()
                );
                Vec::new()
            }
        }
    }

    pub(super) fn read(&self, handle: &str) -> Result<Vec<u8>, GuestError> {
        self.find(handle, FileAccess::Read)?.read()
    }

    pub(super) fn write(&self, handle: &str, body: &[u8]) -> Result<(), GuestError> {
        self.find(handle, FileAccess::ReadWrite)?.write(body)
    }

    fn find(&self, handle: &str, wanted: FileAccess) -> Result<Picked, GuestError> {
        let granted = self.guest.manifest().capabilities().files();
        self.guest.require(
            granted.is_some_and(|granted| granted.covers(wanted)),
            &format!("files = \"{}\"", wanted.as_str()),
        )?;

        let picked = self
            .guest
            .services()
            .shelf()
            .picks()
            .find(self.guest.id(), handle)?
            .ok_or_else(|| {
                GuestError::new(
                    FailureCode::UnknownFile,
                    format!("the person picked no file {handle} for {}", self.guest.id()),
                )
            })?;

        if !picked.access().covers(wanted) {
            return Err(GuestError::not_allowed(format!(
                "{} was picked for {} only",
                picked.name(),
                picked.access().as_str()
            )));
        }

        Ok(picked)
    }
}
