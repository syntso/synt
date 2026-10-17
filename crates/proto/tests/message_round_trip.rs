use synt_proto::Message;
use synt_testkit::TestError;

fn round_trip(text: &str) -> Result<(Message, Message), TestError> {
    let decoded = Message::decode(text.as_bytes())?;
    let again = Message::decode(&decoded.encode()?)?;

    Ok((decoded, again))
}

#[test]
fn a_number_keeps_its_exact_value_through_decode_and_encode() -> Result<(), TestError> {
    let text = r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32600,"message":"x","data":[-33333333333333300000000000000000000000000000000000089924935,0]}}"#;

    let (decoded, again) = round_trip(text)?;

    assert_eq!(again, decoded);

    Ok(())
}

#[test]
fn a_fraction_written_with_many_digits_keeps_its_value() -> Result<(), TestError> {
    let text = r#"{"jsonrpc":"2.0","method":"session.output","params":{"ratio":0.30000000000000004441,"tiny":2.2250738585072011e-308}}"#;

    let (decoded, again) = round_trip(text)?;

    assert_eq!(again, decoded);

    Ok(())
}
