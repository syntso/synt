use super::DocumentInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Written {
    info: DocumentInfo,
    merged: bool,
}

impl Written {
    pub(super) fn new(info: DocumentInfo, merged: bool) -> Self {
        Self { info, merged }
    }

    pub(crate) fn info(&self) -> &DocumentInfo {
        &self.info
    }

    pub(crate) fn merged(&self) -> bool {
        self.merged
    }
}
