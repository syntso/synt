use crate::document::Revision;

pub(super) struct Target {
    pub(super) revision: Revision,
    pub(super) merged: bool,
}

impl Target {
    pub(super) fn head(revision: Revision) -> Self {
        Self {
            revision,
            merged: false,
        }
    }

    pub(super) fn merge(revision: Revision) -> Self {
        Self {
            revision,
            merged: true,
        }
    }
}
