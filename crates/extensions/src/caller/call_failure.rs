#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallFailure {
    pub code: String,
    pub message: String,
}

impl CallFailure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
