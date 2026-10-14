#![cfg(unix)]
#![allow(clippy::unwrap_used)]

use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use nix::fcntl::{FcntlArg, fcntl};
use nix::sys::signal::{SigSet, Signal};
use synt_platform::start_detached;

const SHELL: &str = "/bin/sh";
const LIMIT: Duration = Duration::from_secs(10);
const PAUSE: Duration = Duration::from_millis(20);
const STRAY_FLOOR: i32 = 200;

fn run_shell(script: &str) -> io::Result<()> {
    let arguments = [OsString::from("-c"), OsString::from(script)];

    start_detached(Path::new(SHELL), &arguments)
}

fn wait_for(path: &Path) {
    let deadline = Instant::now() + LIMIT;

    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "{} never appeared",
            path.display()
        );
        thread::sleep(PAUSE);
    }
}

fn place(folder: &tempfile::TempDir, name: &str) -> PathBuf {
    folder.path().join(name)
}

#[test]
fn a_detached_child_does_not_inherit_blocked_signals() {
    let folder = tempfile::tempdir().unwrap();
    let survived = place(&folder, "survived");
    let finished = place(&folder, "finished");
    SigSet::from_iter([Signal::SIGTERM]).thread_block().unwrap();

    run_shell(&format!(
        "{SHELL} -c 'kill -TERM $$; : > \"{}\"'; : > \"{}\"",
        survived.display(),
        finished.display()
    ))
    .unwrap();
    wait_for(&finished);

    assert!(!survived.exists());
}

#[test]
fn a_detached_child_gets_only_the_standard_streams() {
    let folder = tempfile::tempdir().unwrap();
    let listing = place(&folder, "listing");
    let finished = place(&folder, "finished");
    let open = File::open(Path::new("/dev/null")).unwrap();
    let stray = fcntl(&open, FcntlArg::F_DUPFD(STRAY_FLOOR)).unwrap();

    run_shell(&format!(
        "ls /dev/fd > \"{}\"; : > \"{}\"",
        listing.display(),
        finished.display()
    ))
    .unwrap();
    wait_for(&finished);

    let listed = fs::read_to_string(&listing).unwrap();
    let descriptors: Vec<i32> = listed
        .lines()
        .filter_map(|line| line.parse().ok())
        .collect();
    assert!(descriptors.starts_with(&[0, 1, 2]), "{descriptors:?}");
    assert!(!descriptors.contains(&stray), "{descriptors:?}");
}
