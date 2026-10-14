use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TlsMode {
    Disable,
    Prefer,
    Require,
    VerifyFull,
}

impl TlsMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Disable => "disable",
            Self::Prefer => "prefer",
            Self::Require => "require",
            Self::VerifyFull => "verify-full",
        }
    }

    pub(crate) fn verifies(self) -> bool {
        self == Self::VerifyFull
    }
}

impl Display for TlsMode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}
