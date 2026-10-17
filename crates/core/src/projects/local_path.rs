use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::CoreError;

const HOME_PREFIX: &str = "~/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct LocalPath(String);

impl LocalPath {
    pub(crate) fn parse(text: &str) -> Result<Self, CoreError> {
        if text.starts_with(HOME_PREFIX) || PathBuf::from(text).is_absolute() {
            return Ok(Self(text.to_owned()));
        }

        Err(CoreError::NotAbsolute(text.to_owned()))
    }

    pub(crate) fn of_path(path: &Path) -> Result<Self, CoreError> {
        Self::parse(&path.display().to_string())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn path(&self) -> Result<PathBuf, CoreError> {
        let Some(rest) = self.0.strip_prefix(HOME_PREFIX) else {
            return Ok(PathBuf::from(&self.0));
        };

        let home = synt_platform::home_directory().map_err(CoreError::NoHome)?;

        Ok(home.join(rest))
    }
}

impl TryFrom<String> for LocalPath {
    type Error = CoreError;

    fn try_from(text: String) -> Result<Self, CoreError> {
        Self::parse(&text)
    }
}

impl From<LocalPath> for String {
    fn from(path: LocalPath) -> Self {
        path.0
    }
}

#[cfg(test)]
pub(crate) fn rooted(path: &str) -> String {
    if cfg!(windows) {
        format!("C:{path}")
    } else {
        path.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{LocalPath, rooted};

    #[test]
    fn a_local_path_is_absolute_or_under_home() {
        let absolute = rooted("/etc/synt");

        assert_eq!(LocalPath::parse(&absolute).unwrap().as_str(), absolute);
        assert!(
            LocalPath::parse("~/.kube/config")
                .unwrap()
                .path()
                .unwrap()
                .is_absolute()
        );
        assert!(LocalPath::parse("relative/config").is_err());
    }
}
