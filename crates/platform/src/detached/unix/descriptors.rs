use std::fs::{self, File};
use std::os::fd::{AsRawFd, RawFd};

use nix::spawn::PosixSpawnFileActions;

const STANDARD_STREAMS: [RawFd; 3] = [0, 1, 2];
const OPEN_DESCRIPTORS: &str = "/proc/self/fd";

pub(super) trait DetachedDescriptors: Sized {
    fn detached(nowhere: &File) -> nix::Result<Self>;
}

impl DetachedDescriptors for PosixSpawnFileActions {
    fn detached(nowhere: &File) -> nix::Result<Self> {
        let mut actions = Self::init()?;

        for stream in STANDARD_STREAMS {
            actions.add_dup2(nowhere.as_raw_fd(), stream)?;
        }

        for descriptor in inherited_descriptors() {
            actions.add_close(descriptor)?;
        }

        Ok(actions)
    }
}

fn inherited_descriptors() -> Vec<RawFd> {
    if cfg!(target_os = "macos") {
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(OPEN_DESCRIPTORS) else {
        return Vec::new();
    };

    entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_str()?.parse().ok())
        .filter(|descriptor| !STANDARD_STREAMS.contains(descriptor))
        .collect()
}
