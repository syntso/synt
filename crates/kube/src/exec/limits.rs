use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecLimits {
    pub timeout: Duration,
    pub output_bytes: usize,
}

impl Default for ExecLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_mins(10),
            output_bytes: 1024 * 1024,
        }
    }
}
