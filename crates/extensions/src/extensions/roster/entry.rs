use std::sync::Arc;

use crate::extension::Loaded;

pub(super) enum Entry {
    Loaded(Arc<Loaded>),
    Problem(String),
}

impl Entry {
    pub(super) fn loaded(&self) -> Option<&Arc<Loaded>> {
        match self {
            Self::Loaded(loaded) => Some(loaded),
            Self::Problem(_) => None,
        }
    }

    pub(super) fn problem(&self) -> Option<&str> {
        match self {
            Self::Problem(problem) => Some(problem),
            Self::Loaded(_) => None,
        }
    }
}
