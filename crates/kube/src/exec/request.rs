#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExecRequest {
    pub pod: String,
    pub namespace: Option<String>,
    pub container: Option<String>,
    pub command: Vec<String>,
}
