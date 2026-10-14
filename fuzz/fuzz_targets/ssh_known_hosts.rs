#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_ssh::fuzzing::known_hosts;

const HOSTS: [(&str, u16); 4] = [
    ("example.com", 22),
    ("203.0.113.7", 2222),
    ("[::1]", 22),
    ("Ünïcode.example", 22),
];

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    let (presented, file) = text.split_once('\n').unwrap_or(("", text));

    for (host, port) in HOSTS {
        known_hosts(file, host, port, presented);
    }

    let asked = presented.split_whitespace().next().unwrap_or("");
    known_hosts(file, asked, 22, presented);
});
