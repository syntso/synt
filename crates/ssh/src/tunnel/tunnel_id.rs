use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TunnelId(u64);

impl TunnelId {
    pub fn value(self) -> u64 {
        self.0
    }
}

impl From<u64> for TunnelId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl Display for TunnelId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}
