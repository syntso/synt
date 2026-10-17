use synt_journal::JournalError;
use synt_proto::{ErrorCode, RpcError};

use crate::log::log;

pub(super) trait OrUnavailable<T> {
    fn or_unavailable(self) -> Result<T, RpcError>;
}

impl<T> OrUnavailable<T> for Result<T, JournalError> {
    fn or_unavailable(self) -> Result<T, RpcError> {
        self.map_err(|error| {
            log!("journal: {error}");

            RpcError::new(ErrorCode::JOURNAL_UNAVAILABLE, "the journal is unavailable")
        })
    }
}
