use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogText {
    pub text: String,
    pub bytes: usize,
    pub lines: usize,
    pub truncated: bool,
}

impl LogText {
    pub(crate) fn new(text: String, limit: usize) -> Self {
        let bytes = text.len();
        let lines = text.lines().count();

        Self {
            text,
            bytes,
            lines,
            truncated: bytes >= limit,
        }
    }
}
