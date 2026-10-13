#![allow(clippy::unwrap_used)]

mod common;

use std::sync::Arc;

use common::ManualClock;
use hkdf::Hkdf;
use hmac::{Hmac, KeyInit, Mac};
use serde_json::json;
use sha2::Sha256;
use synt_journal::{
    ActionId, Actor, Details, DeviceKey, EventKind, JournalBuilder, JournalId, Link,
    MemoryKeyStore, Name, NewEvent, ResourceRef, Seq,
};

const DEVICE_KEY: [u8; 32] = [7; 32];
const AT: i64 = 1_700_000_000_000;

fn journal_key(id: JournalId) -> [u8; 32] {
    let mut key = [0; 32];
    Hkdf::<Sha256>::new(Some(id.as_bytes()), &DEVICE_KEY)
        .expand(b"synt-journal/v1", &mut key)
        .unwrap();
    key
}

fn hmac(key: &[u8; 32], parts: &[&[u8]]) -> Link {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();

    for part in parts {
        mac.update(part);
    }

    Link::from_bytes(mac.finalize().into_bytes().into())
}

fn prefixed(bytes: &[u8]) -> Vec<u8> {
    let mut out = u32::try_from(bytes.len()).unwrap().to_be_bytes().to_vec();
    out.extend_from_slice(bytes);
    out
}

fn expected_canonical() -> Vec<u8> {
    let actor = [&[3][..], &prefixed(b"agent-1")].concat();
    let details = [
        &[7, 0, 0, 0, 2][..],
        &prefixed(b"command"),
        &[5],
        &prefixed(b"uptime"),
        &prefixed(b"exit"),
        &[3],
        &0_i64.to_be_bytes(),
    ]
    .concat();

    [
        &[1][..],
        &1_u64.to_be_bytes(),
        &AT.to_be_bytes(),
        &prefixed(&actor),
        &[5],
        &[1],
        &41_u64.to_be_bytes(),
        &prefixed(b"ssh.exec"),
        &[1],
        &prefixed(b"@host/api-1"),
        &prefixed(&details),
    ]
    .concat()
}

#[test]
fn links_follow_the_published_formula() {
    let dir = tempfile::tempdir().unwrap();
    let keys = Arc::new(MemoryKeyStore::with_key(DeviceKey::from_bytes(DEVICE_KEY)));
    let clock = Arc::new(ManualClock::starting_at(AT));

    let mut journal = JournalBuilder::new(keys)
        .clock(clock)
        .create(dir.path().join("journal.sqlite"))
        .unwrap();

    let event = NewEvent::new(
        Actor::Agent(Name::new("agent-1").unwrap()),
        EventKind::Finished,
        ActionId::new("ssh.exec").unwrap(),
    )
    .call(Seq::new(41))
    .resource(ResourceRef::new("@host/api-1").unwrap())
    .details(Details::from_serialize(&json!({ "exit": 0, "command": "uptime" })).unwrap());

    let seq = journal.append(&event).unwrap();
    let stored = journal.reader().unwrap().get(seq).unwrap().unwrap();

    let key = journal_key(journal.id());
    let genesis = hmac(&key, &[b"genesis"]);
    let expected = hmac(&key, &[genesis.as_bytes(), &expected_canonical()]);

    assert_eq!(journal.anchors().floor.link, genesis);
    assert_eq!(stored.link(), expected);
}
