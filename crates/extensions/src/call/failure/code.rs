use std::fmt::{self, Display, Formatter};

use synt_proto::ErrorCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureCode {
    Failed,
    NotAllowed,
    Unsupported,
    Unavailable,
    Internal,
    Denied,
    Conflict,
    NotFound,
    TooLarge,
    InvalidRequest,
    InvalidParams,
    InvalidPatch,
    InvalidMessage,
    InvalidKey,
    InvalidTimer,
    InvalidPath,
    UnknownAction,
    UnknownFile,
    UnknownView,
    UnknownPanel,
    UnknownProject,
    StorageFull,
    StorageFailed,
    JournalUnavailable,
    Other(String),
}

impl FailureCode {
    const NAMED: [Self; 24] = [
        Self::Failed,
        Self::NotAllowed,
        Self::Unsupported,
        Self::Unavailable,
        Self::Internal,
        Self::Denied,
        Self::Conflict,
        Self::NotFound,
        Self::TooLarge,
        Self::InvalidRequest,
        Self::InvalidParams,
        Self::InvalidPatch,
        Self::InvalidMessage,
        Self::InvalidKey,
        Self::InvalidTimer,
        Self::InvalidPath,
        Self::UnknownAction,
        Self::UnknownFile,
        Self::UnknownView,
        Self::UnknownPanel,
        Self::UnknownProject,
        Self::StorageFull,
        Self::StorageFailed,
        Self::JournalUnavailable,
    ];

    pub fn as_str(&self) -> &str {
        match self {
            Self::Failed => "failed",
            Self::NotAllowed => "not_allowed",
            Self::Unsupported => "unsupported",
            Self::Unavailable => "unavailable",
            Self::Internal => "internal",
            Self::Denied => "denied",
            Self::Conflict => "conflict",
            Self::NotFound => "not_found",
            Self::TooLarge => "too_large",
            Self::InvalidRequest => "invalid_request",
            Self::InvalidParams => "invalid_params",
            Self::InvalidPatch => "invalid_patch",
            Self::InvalidMessage => "invalid_message",
            Self::InvalidKey => "invalid_key",
            Self::InvalidTimer => "invalid_timer",
            Self::InvalidPath => "invalid_path",
            Self::UnknownAction => "unknown_action",
            Self::UnknownFile => "unknown_file",
            Self::UnknownView => "unknown_view",
            Self::UnknownPanel => "unknown_panel",
            Self::UnknownProject => "unknown_project",
            Self::StorageFull => "storage_full",
            Self::StorageFailed => "storage_failed",
            Self::JournalUnavailable => "journal_unavailable",
            Self::Other(code) => code,
        }
    }
}

impl From<String> for FailureCode {
    fn from(code: String) -> Self {
        Self::NAMED
            .into_iter()
            .find(|named| named.as_str() == code)
            .unwrap_or(Self::Other(code))
    }
}

impl From<ErrorCode> for FailureCode {
    fn from(code: ErrorCode) -> Self {
        match code {
            ErrorCode::DENIED => Self::Denied,
            ErrorCode::UNKNOWN_ACTION => Self::UnknownAction,
            ErrorCode::INVALID_PARAMS => Self::InvalidParams,
            ErrorCode::ACTION_FAILED => Self::Failed,
            ErrorCode::JOURNAL_UNAVAILABLE => Self::JournalUnavailable,
            _ => Self::Internal,
        }
    }
}

impl Display for FailureCode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use synt_proto::ErrorCode;

    use super::FailureCode;

    #[test]
    fn named_codes_round_trip_and_others_are_kept() {
        for code in FailureCode::NAMED {
            assert_eq!(FailureCode::from(code.as_str().to_owned()), code);
        }

        assert_eq!(
            FailureCode::from("boom".to_owned()),
            FailureCode::Other("boom".to_owned())
        );
    }

    #[test]
    fn rpc_codes_map_to_their_names() {
        assert_eq!(FailureCode::from(ErrorCode::DENIED), FailureCode::Denied);
        assert_eq!(
            FailureCode::from(ErrorCode::ACTION_FAILED).as_str(),
            "failed"
        );
        assert_eq!(
            FailureCode::from(ErrorCode::PARSE_ERROR),
            FailureCode::Internal
        );
    }
}
