use super::DocumentInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Opened {
    info: DocumentInfo,
    body: Vec<u8>,
}

impl Opened {
    pub(super) fn new(info: DocumentInfo, body: Vec<u8>) -> Self {
        Self { info, body }
    }

    pub(crate) fn info(&self) -> &DocumentInfo {
        &self.info
    }

    pub(crate) fn into_body(self) -> Vec<u8> {
        self.body
    }
}
