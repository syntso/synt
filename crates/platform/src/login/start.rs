use std::path::Path;

use super::{LoginStatus, Os, Program, Registration};
use crate::PlatformError;

pub struct LoginStart {
    registration: Os,
}

impl LoginStart {
    pub fn for_this_user() -> Result<Self, PlatformError> {
        let registration = Os::for_this_user()?;

        Ok(Self { registration })
    }

    pub fn install(&self, program: &Path) -> Result<(), PlatformError> {
        let program = Program::checked(program)?;

        self.registration.install(&program)
    }

    pub fn uninstall(&self) -> Result<bool, PlatformError> {
        self.registration.uninstall()
    }

    pub fn status(&self) -> Result<LoginStatus, PlatformError> {
        self.registration.status()
    }
}
