use super::Visit;
use crate::interface::GuestError;
use crate::manifest::FileAccess;
use crate::picked::Picked;

impl Visit {
    pub(super) fn picked_files(&self) -> Vec<Picked> {
        if self.manifest().capabilities.files.is_none() {
            return Vec::new();
        }

        self.guest
            .services()
            .shelf
            .picks(self.id())
            .unwrap_or_default()
    }

    pub(super) fn read_picked(&self, handle: &str) -> Result<Vec<u8>, GuestError> {
        self.find_pick(handle, FileAccess::Read)?.read()
    }

    pub(super) fn write_picked(&self, handle: &str, body: &[u8]) -> Result<(), GuestError> {
        self.find_pick(handle, FileAccess::ReadWrite)?.write(body)
    }

    fn find_pick(&self, handle: &str, wanted: FileAccess) -> Result<Picked, GuestError> {
        let granted = self.manifest().capabilities.files;
        self.allowed(
            granted.is_some_and(|granted| granted.covers(wanted)),
            &format!("files = \"{}\"", wanted.as_str()),
        )?;

        let picked = self
            .guest
            .services()
            .shelf
            .pick(self.id(), handle)?
            .ok_or_else(|| {
                GuestError::new(
                    "unknown_file",
                    format!("the person picked no file {handle} for {}", self.id()),
                )
            })?;

        if !picked.access.covers(wanted) {
            return Err(GuestError::not_allowed(format!(
                "{} was picked for {} only",
                picked.name,
                picked.access.as_str()
            )));
        }

        Ok(picked)
    }
}
