use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct IndexSeal {
    pub(crate) serial: u64,
    pub(crate) expires: u64,
}
