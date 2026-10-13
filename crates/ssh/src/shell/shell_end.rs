#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShellEnd {
    pub exit: Option<u32>,
    pub signal: Option<String>,
    pub lost: bool,
}
