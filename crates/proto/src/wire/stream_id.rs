use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StreamId(u32);

impl StreamId {
    pub const FIRST_OPENED_BY_CLIENT: Self = Self(1 << 31);

    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub fn is_opened_by_client(self) -> bool {
        self >= Self::FIRST_OPENED_BY_CLIENT
    }
}
