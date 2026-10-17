use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::Deserialize;

use crate::KubeError;

const LONGEST: usize = 253;
const FORBIDDEN: [char; 5] = ['/', '?', '#', '%', '\\'];

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(try_from = "String")]
pub struct ObjectName(String);

impl ObjectName {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn is_valid(text: &str) -> bool {
        let blank = |character: char| character.is_whitespace() || character.is_control();

        !text.is_empty()
            && text.len() <= LONGEST
            && text != "."
            && text != ".."
            && !text.contains(FORBIDDEN)
            && !text.contains(blank)
    }
}

impl TryFrom<String> for ObjectName {
    type Error = KubeError;

    fn try_from(text: String) -> Result<Self, KubeError> {
        if !Self::is_valid(&text) {
            return Err(KubeError::invalid(format!(
                "an object name must be 1 to {LONGEST} characters without '/', '?', '#', '%' \
                 or spaces, got {text:?}"
            )));
        }

        Ok(Self(text))
    }
}

impl TryFrom<&str> for ObjectName {
    type Error = KubeError;

    fn try_from(text: &str) -> Result<Self, KubeError> {
        Self::try_from(text.to_owned())
    }
}

impl Display for ObjectName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectName;

    #[test]
    fn ordinary_names_pass() {
        for text in ["api-7d9f8b-x2x4q", "system:controller:job"] {
            assert_eq!(ObjectName::try_from(text).unwrap().as_str(), text);
        }
    }

    #[test]
    fn names_that_would_change_the_request_path_are_refused() {
        for text in [
            "",
            ".",
            "..",
            "a/b",
            "a?watch=1",
            "a#b",
            "a%2Fb",
            "a b",
            "a\\b",
        ] {
            assert!(ObjectName::try_from(text).is_err(), "{text:?}");
        }
    }
}
