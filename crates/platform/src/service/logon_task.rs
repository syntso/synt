use std::env;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use super::tool;
use crate::native::system_root;
use crate::{LoginStatus, PlatformError};

const TASK: &str = "Synt Core";

pub(super) struct LogonTask {
    name: String,
    schtasks: PathBuf,
}

impl LogonTask {
    pub(super) fn for_this_user() -> Result<Self, PlatformError> {
        let user = env::var("USERNAME")
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "USERNAME is not set"))?;

        Ok(Self {
            name: format!("{TASK} ({user})"),
            schtasks: system_root().join("System32").join("schtasks.exe"),
        })
    }

    pub(super) fn install(&self, program: &Path) -> Result<(), PlatformError> {
        let mut run = OsString::from("\"");
        run.push(program.as_os_str());
        run.push("\"");

        let arguments: [OsString; 10] = [
            "/Create".into(),
            "/F".into(),
            "/TN".into(),
            self.name.clone().into(),
            "/SC".into(),
            "ONLOGON".into(),
            "/RL".into(),
            "LIMITED".into(),
            "/TR".into(),
            run,
        ];
        tool::require(&self.schtasks, arguments)?;

        Ok(())
    }

    pub(super) fn uninstall(&self) -> Result<bool, PlatformError> {
        if !self.exists()? {
            return Ok(false);
        }

        tool::require(&self.schtasks, ["/Delete", "/F", "/TN", self.name.as_str()])?;

        Ok(true)
    }

    pub(super) fn status(&self) -> Result<LoginStatus, PlatformError> {
        if !self.exists()? {
            return Ok(LoginStatus::NotRegistered);
        }

        Ok(LoginStatus::Registered {
            place: format!("logon task \"{}\"", self.name),
            program: None,
        })
    }

    fn exists(&self) -> Result<bool, PlatformError> {
        tool::succeeds(&self.schtasks, ["/Query", "/TN", self.name.as_str()])
    }
}
