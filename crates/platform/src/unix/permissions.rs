use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;

use nix::unistd;

const GROUP_OR_OTHER_WRITE: u32 = 0o022;

pub(crate) fn writable_by_others(metadata: &Metadata) -> bool {
    metadata.mode() & GROUP_OR_OTHER_WRITE != 0
}

pub(crate) fn owned_by_current_user(metadata: &Metadata) -> bool {
    metadata.uid() == unistd::geteuid().as_raw()
}
