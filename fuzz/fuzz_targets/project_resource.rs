#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_core::fuzzing::{read_resource, write_resource};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(resource) = read_resource(text) else {
        return;
    };

    let written = write_resource(&resource).unwrap();
    let again = read_resource(&written).unwrap();

    assert_eq!(again, resource);
});
