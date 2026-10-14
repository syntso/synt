#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PtyEnd {
    pub exit: Option<u32>,
    pub signal: Option<String>,
}
