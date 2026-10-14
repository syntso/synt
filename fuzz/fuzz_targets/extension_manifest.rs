#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_extensions::Manifest;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(manifest) = Manifest::parse(text) else {
        return;
    };

    let written = manifest.to_toml().unwrap();
    let again = Manifest::parse(&written).unwrap();

    assert_eq!(again, manifest);
});
