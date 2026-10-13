use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self { major: 1, minor: 0 };

    pub fn speaks_with(self, other: Self) -> bool {
        self.major == other.major
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use super::ProtocolVersion;

    #[test]
    fn minor_versions_are_compatible() {
        let newer = ProtocolVersion { major: 1, minor: 4 };
        assert!(ProtocolVersion::CURRENT.speaks_with(newer));
    }

    #[test]
    fn major_versions_are_not() {
        let next = ProtocolVersion { major: 2, minor: 0 };
        assert!(!ProtocolVersion::CURRENT.speaks_with(next));
    }
}
