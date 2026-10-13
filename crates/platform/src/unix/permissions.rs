use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;

const GROUP_OR_OTHER_WRITE: u32 = 0o022;

pub(crate) fn writable_by_others(metadata: &Metadata) -> bool {
    metadata.mode() & GROUP_OR_OTHER_WRITE != 0
}
