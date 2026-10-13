#![allow(clippy::unwrap_used)]

mod common;

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{env, fs, thread};

use common::{FileKeyStore, exec};
use synt_journal::{Journal, JournalId};

const DIR_VARIABLE: &str = "SYNT_JOURNAL_CRASH_DIR";

#[test]
#[ignore = "runs only as the child process of killed_mid_append_leaves_an_intact_journal"]
fn crash_child() {
    let dir = PathBuf::from(env::var(DIR_VARIABLE).unwrap());
    let keys = Arc::new(FileKeyStore::new(&dir));

    let mut journal = Journal::create(dir.join("journal.sqlite"), keys).unwrap();
    fs::write(dir.join("id"), journal.id().as_bytes()).unwrap();

    let mut n = 0;

    loop {
        journal
            .append_all(&[exec(&format!("{n}a")), exec(&format!("{n}b"))])
            .unwrap();
        n += 1;
    }
}

#[test]
fn killed_mid_append_leaves_an_intact_journal() {
    let dir = tempfile::tempdir().unwrap();

    let mut child = Command::new(env::current_exe().unwrap())
        .args(["--exact", "crash_child", "--ignored", "--nocapture"])
        .env(DIR_VARIABLE, dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let id_path = dir.path().join("id");
    let started = Instant::now();

    while !id_path.exists() && started.elapsed() < Duration::from_secs(20) {
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_millis(300));

    child.kill().unwrap();
    child.wait().unwrap();

    let id = JournalId::from_bytes(fs::read(&id_path).unwrap().try_into().unwrap());
    let keys = Arc::new(FileKeyStore::new(dir.path()));

    let journal = Journal::open(dir.path().join("journal.sqlite"), id, keys).unwrap();
    let verification = journal.reader().unwrap().verify().unwrap();

    assert!(verification.is_intact(), "{verification:?}");
    assert!(journal.anchors().head.seq.get() > 0);
}
