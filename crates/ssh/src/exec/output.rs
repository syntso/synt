use crate::Algorithms;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecOutput {
    pub exit: Option<u32>,
    pub signal: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
    pub duration_ms: u64,
    pub algorithms: Option<Algorithms>,
}
