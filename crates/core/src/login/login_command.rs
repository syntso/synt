use std::env;
use std::ffi::OsString;

use synt_platform::{LoginStart, PlatformError};

use crate::CoreError;

const INSTALL: &str = "install";
const UNINSTALL: &str = "uninstall";
const STATUS: &str = "status";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginCommand {
    Install,
    Uninstall,
    Status,
}

impl LoginCommand {
    pub const NAME: &str = "service";

    pub fn parse(arguments: &[OsString]) -> Result<Self, CoreError> {
        let [command] = arguments else {
            return Err(usage());
        };

        match command.to_str() {
            Some(INSTALL) => Ok(Self::Install),
            Some(UNINSTALL) => Ok(Self::Uninstall),
            Some(STATUS) => Ok(Self::Status),
            _ => Err(usage()),
        }
    }

    pub fn run(self) -> Result<String, CoreError> {
        self.perform().map_err(CoreError::LoginStart)
    }

    fn perform(self) -> Result<String, PlatformError> {
        let login = LoginStart::for_this_user()?;

        match self {
            Self::Install => install(&login),
            Self::Uninstall => uninstall(&login),
            Self::Status => Ok(login.status()?.to_string()),
        }
    }
}

fn install(login: &LoginStart) -> Result<String, PlatformError> {
    let program = env::current_exe()?.canonicalize()?;

    login.install(&program)?;

    Ok(login.status()?.to_string())
}

fn uninstall(login: &LoginStart) -> Result<String, PlatformError> {
    let removed = login.uninstall()?;

    let message = if removed {
        "removed: the core no longer starts at login"
    } else {
        "nothing to remove: the core does not start at login"
    };

    Ok(message.to_owned())
}

fn usage() -> CoreError {
    CoreError::Usage(format!(
        "{} needs one of {INSTALL}, {UNINSTALL}, {STATUS}",
        LoginCommand::NAME
    ))
}
