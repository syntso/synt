use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::ExtensionError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssetPath(String);

impl AssetPath {
    pub(crate) const LONGEST: usize = 200;

    pub(crate) fn is_plain_segment(segment: &str) -> bool {
        let allowed =
            |character: char| character.is_ascii_alphanumeric() || "-_.".contains(character);

        !segment.is_empty() && !segment.starts_with('.') && segment.chars().all(allowed)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for AssetPath {
    type Error = ExtensionError;

    fn try_from(text: String) -> Result<Self, ExtensionError> {
        let plain = text.split('/').all(Self::is_plain_segment);

        if text.len() > Self::LONGEST || !plain {
            return Err(ExtensionError::AssetPath(text));
        }

        Ok(Self(text))
    }
}

impl From<AssetPath> for String {
    fn from(value: AssetPath) -> Self {
        value.0
    }
}

impl Borrow<str> for AssetPath {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Display for AssetPath {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::AssetPath;

    #[test]
    fn asset_paths_stay_inside_the_bundle() {
        for path in ["index.html", "assets/app-1.2.js"] {
            assert!(AssetPath::try_from(path.to_owned()).is_ok(), "{path:?}");
        }

        for path in [
            "",
            "/index.html",
            "../x",
            "a//b",
            ".env",
            "a/.hidden",
            "a\\b",
            "a b",
        ] {
            assert!(AssetPath::try_from(path.to_owned()).is_err(), "{path:?}");
        }
    }
}
