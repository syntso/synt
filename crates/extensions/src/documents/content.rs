use super::DocumentInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Document {
    pub(crate) info: DocumentInfo,
    pub(crate) body: Vec<u8>,
}
