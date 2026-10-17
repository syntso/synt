use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct IndexSeal {
    serial: u64,
    expires: u64,
}

impl IndexSeal {
    pub(crate) fn serial(self) -> u64 {
        self.serial
    }

    pub(crate) fn expired_at(self, now_seconds: u64) -> bool {
        self.expires <= now_seconds
    }
}
