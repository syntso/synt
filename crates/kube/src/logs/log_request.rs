#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogRequest {
    pub pod: String,
    pub namespace: Option<String>,
    pub container: Option<String>,
    pub tail_lines: Option<u32>,
    pub since_seconds: Option<u32>,
    pub previous: bool,
    pub timestamps: bool,
}
