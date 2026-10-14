#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuestError {
    pub(crate) code: String,
    pub(crate) message: String,
}

impl GuestError {
    pub(crate) fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub(crate) fn not_allowed(message: impl Into<String>) -> Self {
        Self::new("not_allowed", message)
    }

    pub(crate) fn unsupported(entry: &str) -> Self {
        Self::new(
            "unsupported",
            format!("extensions for api 0.1 have no {entry}"),
        )
    }
}
