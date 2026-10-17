#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_journal::fuzzing::{actor_bytes, decode_actor};

fuzz_target!(|data: &[u8]| {
    let Some(actor) = decode_actor(data) else {
        return;
    };

    assert_eq!(actor_bytes(&actor), data);
});
