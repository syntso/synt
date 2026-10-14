mod code;

pub use code::FailureCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallFailure {
    code: FailureCode,
    message: String,
}

impl CallFailure {
    pub fn new(code: FailureCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> &FailureCode {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}
