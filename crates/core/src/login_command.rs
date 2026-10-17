use std::env;
use std::ffi::OsString;

use synt_platform::{LoginStart, PlatformError};

use crate::CoreError;
use crate::blocking::blocking;

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

    pub fn from_args(
        arguments: impl IntoIterator<Item = OsString>,
    ) -> Result<Option<Self>, CoreError> {
        let arguments: Vec<OsString> = arguments.into_iter().collect();

        let Some((first, rest)) = arguments.split_first() else {
            return Ok(None);
        };
        if first != Self::NAME {
            return Ok(None);
        }

        Ok(Some(Self::parse(rest)?))
    }

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

    pub async fn run(self) -> Result<String, CoreError> {
        blocking(move || self.perform().map_err(CoreError::LoginStart)).await
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
