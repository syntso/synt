use super::DocumentInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Written {
    pub(crate) info: DocumentInfo,
    pub(crate) merged: bool,
}
