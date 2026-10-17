use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestSeq(u64);

impl RequestSeq {
    pub const fn new(seq: u64) -> Self {
        Self(seq)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RequestSeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
