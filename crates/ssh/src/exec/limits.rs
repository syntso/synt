use std::time::Duration;

const DEFAULT_OUTPUT_BYTES: usize = 1024 * 1024;
const DEFAULT_TIMEOUT: Duration = Duration::from_mins(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecLimits {
    pub output_bytes: usize,
    pub timeout: Duration,
}

impl Default for ExecLimits {
    fn default() -> Self {
        Self {
            output_bytes: DEFAULT_OUTPUT_BYTES,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}
