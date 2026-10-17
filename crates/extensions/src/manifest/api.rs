use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::ManifestError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Api {
    V0_1,
    V0_2,
}

impl Api {
    pub const SUPPORTED: [Self; 2] = [Self::V0_1, Self::V0_2];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::V0_1 => "0.1",
            Self::V0_2 => "0.2",
        }
    }

    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        Self::SUPPORTED
            .into_iter()
            .find(|api| api.as_str() == text)
            .ok_or_else(|| Self::unsupported(text))
    }

    fn unsupported(text: &str) -> ManifestError {
        let supported: Vec<&str> = Self::SUPPORTED.iter().map(|api| api.as_str()).collect();

        ManifestError::UnsupportedApi {
            api: text.to_owned(),
            supported: supported.join(" and "),
        }
    }
}

impl TryFrom<String> for Api {
    type Error = ManifestError;

    fn try_from(text: String) -> Result<Self, ManifestError> {
        Self::parse(&text)
    }
}

impl From<Api> for String {
    fn from(api: Api) -> Self {
        api.as_str().to_owned()
    }
}

impl Display for Api {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::Api;

    #[test]
    fn both_versions_round_trip() {
        for api in Api::SUPPORTED {
            assert_eq!(Api::parse(api.as_str()).unwrap(), api);
        }
    }

    #[test]
    fn other_versions_name_the_supported_ones() {
        let refusal = Api::parse("9.0").unwrap_err().to_string();

        assert!(refusal.contains("0.1 and 0.2"), "{refusal}");
    }
}
