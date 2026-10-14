use std::env;
use std::ffi::OsString;
use std::io;

use super::{LoginStatus, Program, Registration, Tool};
use crate::PlatformError;
use crate::directories::system_root;

const TASK: &str = "Synt Core";

pub(super) struct LogonTask {
    name: String,
    schtasks: Tool,
}

impl LogonTask {
    fn exists(&self) -> Result<bool, PlatformError> {
        self.schtasks
            .succeeds(["/Query", "/TN", self.name.as_str()])
    }
}

impl Registration for LogonTask {
    fn for_this_user() -> Result<Self, PlatformError> {
        let user = env::var("USERNAME")
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "USERNAME is not set"))?;

        Ok(Self {
            name: format!("{TASK} ({user})"),
            schtasks: Tool::new(system_root().join("System32").join("schtasks.exe")),
        })
    }

    fn install(&self, program: &Program) -> Result<(), PlatformError> {
        let mut run = OsString::from("\"");
        run.push(program.path().as_os_str());
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
        self.schtasks.require(arguments)?;

        Ok(())
    }

    fn uninstall(&self) -> Result<bool, PlatformError> {
        if !self.exists()? {
            return Ok(false);
        }

        self.schtasks
            .require(["/Delete", "/F", "/TN", self.name.as_str()])?;

        Ok(true)
    }

    fn status(&self) -> Result<LoginStatus, PlatformError> {
        if !self.exists()? {
            return Ok(LoginStatus::NotRegistered);
        }

        Ok(LoginStatus::Registered {
            place: format!("logon task \"{}\"", self.name),
            program: None,
        })
    }
}
