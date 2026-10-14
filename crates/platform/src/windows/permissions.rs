use std::fs::Metadata;

pub(crate) fn writable_by_others(_metadata: &Metadata) -> bool {
    false
}

pub(crate) fn owned_by_current_user(_metadata: &Metadata) -> bool {
    true
}
