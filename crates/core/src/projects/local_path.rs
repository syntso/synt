use std::path::PathBuf;

use crate::CoreError;

const HOME_PREFIX: &str = "~/";

pub(crate) struct LocalPath(PathBuf);

impl LocalPath {
    pub(crate) fn parse(text: &str) -> Result<Self, CoreError> {
        if let Some(rest) = text.strip_prefix(HOME_PREFIX) {
            let home = synt_platform::home_directory().map_err(CoreError::NoHome)?;

            return Ok(Self(home.join(rest)));
        }

        let path = PathBuf::from(text);

        if !path.is_absolute() {
            return Err(CoreError::NotAbsolute(text.to_owned()));
        }

        Ok(Self(path))
    }

    pub(crate) fn into_path(self) -> PathBuf {
        self.0
    }
}
