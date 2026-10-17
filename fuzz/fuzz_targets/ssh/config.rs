#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_ssh::fuzzing::config;

fuzz_target!(|data: &[u8]| {
    if data
        .to_ascii_lowercase()
        .windows(7)
        .any(|window| window == b"include")
    {
        return;
    }

    for host in ["example", "web-1.prod", "*", ""] {
        config(data, host);
    }
});
