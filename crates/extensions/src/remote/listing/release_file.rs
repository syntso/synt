use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::ExtensionError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct ReleaseFile(String);

impl ReleaseFile {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ReleaseFile {
    type Error = ExtensionError;

    fn try_from(name: String) -> Result<Self, ExtensionError> {
        let plain = name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character));

        if name.is_empty() || name.starts_with('.') || !plain {
            return Err(ExtensionError::ReleaseFile(name));
        }

        Ok(Self(name))
    }
}

impl From<ReleaseFile> for String {
    fn from(file: ReleaseFile) -> Self {
        file.0
    }
}

impl Display for ReleaseFile {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::ReleaseFile;

    #[test]
    fn file_names_stay_inside_the_release() {
        assert!(ReleaseFile::try_from("uptime-0.1.0.wasm".to_owned()).is_ok());

        for name in ["", ".hidden", "../index.json", "a/b.wasm", "https://x"] {
            assert!(ReleaseFile::try_from(name.to_owned()).is_err(), "{name}");
        }
    }
}
