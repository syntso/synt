use std::fs::Metadata;

pub(crate) fn writable_by_others(_metadata: &Metadata) -> bool {
    false
}
