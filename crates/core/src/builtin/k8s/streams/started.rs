use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct StreamStarted {
    pub(crate) request: u64,
    pub(crate) stream: u32,
}
