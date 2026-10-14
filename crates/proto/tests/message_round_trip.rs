#![allow(clippy::unwrap_used)]

use synt_proto::Message;

fn round_trip(text: &str) -> (Message, Message) {
    let decoded = Message::decode(text.as_bytes()).unwrap();
    let again = Message::decode(&decoded.encode().unwrap()).unwrap();

    (decoded, again)
}

#[test]
fn a_number_keeps_its_exact_value_through_decode_and_encode() {
    let text = r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32600,"message":"x","data":[-33333333333333300000000000000000000000000000000000089924935,0]}}"#;

    let (decoded, again) = round_trip(text);

    assert_eq!(again, decoded);
}

#[test]
fn a_fraction_written_with_many_digits_keeps_its_value() {
    let text = r#"{"jsonrpc":"2.0","method":"session.output","params":{"ratio":0.30000000000000004441,"tiny":2.2250738585072011e-308}}"#;

    let (decoded, again) = round_trip(text);

    assert_eq!(again, decoded);
}
