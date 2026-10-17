use std::env;
use std::ffi::OsString;

use synt_platform::{LoginStart, PlatformError};

use crate::CoreError;

const INSTALL: &str = "install";
const UNINSTALL: &str = "uninstall";
const STATUS: &str = "status";
const REMOVED: &str = "removed: the core no longer starts at login";
const NOTHING_TO_REMOVE: &str = "nothing to remove: the core does not start at login";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginCommand {
    Install,
    Uninstall,
    Status,
}

impl LoginCommand {
    pub const NAME: &str = "service";

    pub fn parse(arguments: &[OsString]) -> Result<Self, CoreError> {
        let usage = CoreError::CommandUsage {
            command: Self::NAME,
            choices: &[INSTALL, UNINSTALL, STATUS],
        };

        let [command] = arguments else {
            return Err(usage);
        };

        match command.to_str() {
            Some(INSTALL) => Ok(Self::Install),
            Some(UNINSTALL) => Ok(Self::Uninstall),
            Some(STATUS) => Ok(Self::Status),
            _ => Err(usage),
        }
    }

    pub fn run(self) -> Result<String, CoreError> {
        self.perform().map_err(CoreError::LoginStart)
    }

    fn perform(self) -> Result<String, PlatformError> {
        let login = LoginStart::for_this_user()?;

        match self {
            Self::Install => Self::install(&login),
            Self::Uninstall => Self::uninstall(&login),
            Self::Status => Ok(login.status()?.to_string()),
        }
    }

    fn install(login: &LoginStart) -> Result<String, PlatformError> {
        let program = env::current_exe()?.canonicalize()?;

        login.install(&program)?;

        Ok(login.status()?.to_string())
    }

    fn uninstall(login: &LoginStart) -> Result<String, PlatformError> {
        let message = if login.uninstall()? {
            REMOVED
        } else {
            NOTHING_TO_REMOVE
        };

        Ok(message.to_owned())
    }
}
