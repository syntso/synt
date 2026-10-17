use std::ffi::{OsStr, OsString};
use std::path::{self, PathBuf};
use std::time::Duration;

use synt_platform::{Endpoint, WEAK_VAULT_FLAG, WeakVault, data_directory};

use crate::CoreError;

const ENDPOINT_FLAG: &str = "--endpoint";
const DATA_FLAG: &str = "--data";
const STAY_FLAG: &str = "--stay";
const IDLE_FLAG: &str = "--idle-minutes";
const VALUE_FLAGS: [&str; 3] = [ENDPOINT_FLAG, DATA_FLAG, IDLE_FLAG];
const IDLE_LIMIT: Duration = Duration::from_mins(10);

#[derive(Debug)]
pub struct Arguments {
    endpoint: Option<Endpoint>,
    data: Option<PathBuf>,
    weak_vault: WeakVault,
    idle: Option<Duration>,
}

impl Arguments {
    pub fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Self, CoreError> {
        let mut parsed = Self {
            endpoint: None,
            data: None,
            weak_vault: WeakVault::Refuse,
            idle: Some(IDLE_LIMIT),
        };
        let mut arguments = arguments.into_iter();

        while let Some(argument) = arguments.next() {
            if parsed.switch(&argument) {
                continue;
            }

            if !VALUE_FLAGS.iter().any(|flag| argument == *flag) {
                let unknown = argument.to_string_lossy().into_owned();
                return Err(CoreError::UnknownArgument(unknown));
            }

            let value = arguments
                .next()
                .ok_or_else(|| CoreError::MissingValue(argument.to_string_lossy().into_owned()))?;

            parsed.set(&argument, value)?;
        }

        Ok(parsed)
    }

    pub fn endpoint(&self) -> Result<Endpoint, CoreError> {
        match &self.endpoint {
            Some(endpoint) => Ok(endpoint.clone()),
            None => Endpoint::for_user().map_err(CoreError::Endpoint),
        }
    }

    pub fn data(&self) -> Result<PathBuf, CoreError> {
        match &self.data {
            Some(data) => Ok(path::absolute(data)?),
            None => Ok(data_directory()?),
        }
    }

    pub fn weak_vault(&self) -> WeakVault {
        self.weak_vault
    }

    pub fn idle(&self) -> Option<Duration> {
        self.idle
    }

    fn switch(&mut self, argument: &OsStr) -> bool {
        if argument == WEAK_VAULT_FLAG {
            self.weak_vault = WeakVault::Allow;
            return true;
        }

        if argument == STAY_FLAG {
            self.idle = None;
            return true;
        }

        false
    }

    fn set(&mut self, argument: &OsStr, value: OsString) -> Result<(), CoreError> {
        if argument == ENDPOINT_FLAG {
            self.endpoint = Some(Endpoint::new(value));
        } else if argument == DATA_FLAG {
            self.data = Some(PathBuf::from(value));
        } else {
            self.set_idle(&value)?;
        }

        Ok(())
    }

    fn set_idle(&mut self, value: &OsStr) -> Result<(), CoreError> {
        let minutes: u64 = value
            .to_str()
            .and_then(|text| text.parse().ok())
            .ok_or(CoreError::NotMinutes(IDLE_FLAG))?;

        if self.idle.is_some() {
            self.idle = Some(Duration::from_mins(minutes));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<Arguments, CoreError> {
        Arguments::parse(arguments.iter().map(OsString::from))
    }

    #[test]
    fn stay_keeps_the_core_running_whatever_the_idle_limit() {
        let parsed = parse(&[STAY_FLAG, IDLE_FLAG, "3"]).unwrap();

        assert_eq!(parsed.idle(), None);
    }

    #[test]
    fn idle_minutes_set_the_limit() {
        let parsed = parse(&[IDLE_FLAG, "3"]).unwrap();

        assert_eq!(parsed.idle(), Some(Duration::from_mins(3)));
    }

    #[test]
    fn unknown_and_unfinished_arguments_are_refused() {
        assert!(matches!(
            parse(&["--nope"]),
            Err(CoreError::UnknownArgument(_))
        ));
        assert!(matches!(
            parse(&[DATA_FLAG]),
            Err(CoreError::MissingValue(_))
        ));
        assert!(matches!(
            parse(&[IDLE_FLAG, "soon"]),
            Err(CoreError::NotMinutes(_))
        ));
    }
}
