use std::path::Path;

use crate::service::program_check::check_program;
use crate::{LoginStatus, PlatformError};

#[cfg(target_os = "macos")]
use crate::service::launch_agent::LaunchAgent as Registration;
#[cfg(windows)]
use crate::service::logon_task::LogonTask as Registration;
#[cfg(all(unix, not(target_os = "macos")))]
use crate::service::systemd_unit::SystemdUnit as Registration;

pub struct LoginStart {
    registration: Registration,
}

impl LoginStart {
    pub fn for_this_user() -> Result<Self, PlatformError> {
        let registration = Registration::for_this_user()?;

        Ok(Self { registration })
    }

    pub fn install(&self, program: &Path) -> Result<(), PlatformError> {
        check_program(program)?;

        self.registration.install(program)
    }

    pub fn uninstall(&self) -> Result<bool, PlatformError> {
        self.registration.uninstall()
    }

    pub fn status(&self) -> Result<LoginStatus, PlatformError> {
        self.registration.status()
    }
}
