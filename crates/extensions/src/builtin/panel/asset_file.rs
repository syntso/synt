use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Asset {
    pub path: String,
    pub mime: &'static str,
    pub bytes: usize,
    pub data: String,
}
