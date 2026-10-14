use std::fmt::{self, Display, Formatter};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    Read,
    Write,
    Privileged,
}

impl Effect {
    pub fn name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Privileged => "privileged",
        }
    }
}

impl Display for Effect {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}
