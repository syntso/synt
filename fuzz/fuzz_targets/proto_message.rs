#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_proto::Message;

fuzz_target!(|data: &[u8]| {
    let Ok(message) = Message::decode(data) else {
        return;
    };

    let encoded = message.encode().unwrap();
    let again = Message::decode(&encoded).unwrap();

    assert_eq!(again, message);
});
