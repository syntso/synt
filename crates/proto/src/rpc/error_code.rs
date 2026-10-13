use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ErrorCode(pub i64);

impl ErrorCode {
    pub const PARSE_ERROR: Self = Self(-32_700);
    pub const INVALID_REQUEST: Self = Self(-32_600);
    pub const METHOD_NOT_FOUND: Self = Self(-32_601);
    pub const INVALID_PARAMS: Self = Self(-32_602);
    pub const INTERNAL: Self = Self(-32_603);

    pub const HANDSHAKE_REQUIRED: Self = Self(-32_000);
    pub const PROTOCOL_MISMATCH: Self = Self(-32_001);
    pub const UNKNOWN_ACTION: Self = Self(-32_010);
    pub const ACTION_FAILED: Self = Self(-32_011);
    pub const DENIED: Self = Self(-32_012);
    pub const JOURNAL_UNAVAILABLE: Self = Self(-32_013);
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
