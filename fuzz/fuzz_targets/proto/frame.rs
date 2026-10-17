#![no_main]

use bytes::BytesMut;
use libfuzzer_sys::fuzz_target;
use synt_proto::{Frame, FrameCodec};
use tokio_util::codec::{Decoder, Encoder};

fuzz_target!(|data: &[u8]| {
    let (whole, split) = synt_fuzz::pieces(data);
    let (at_once, refused) = decode_all(&[whole]);
    let (in_pieces, refused_in_pieces) = decode_all(&split);

    assert_eq!(at_once, in_pieces);
    assert_eq!(refused, refused_in_pieces);

    for frame in at_once {
        let mut encoded = BytesMut::new();
        FrameCodec::new()
            .encode(frame.clone(), &mut encoded)
            .unwrap();

        let again = FrameCodec::new().decode(&mut encoded).unwrap();
        assert_eq!(again, Some(frame));
        assert!(encoded.is_empty());
    }
});

fn decode_all(chunks: &[&[u8]]) -> (Vec<Frame>, bool) {
    let mut codec = FrameCodec::with_limit(64 * 1024);
    let mut buffer = BytesMut::new();
    let mut frames = Vec::new();

    for chunk in chunks {
        buffer.extend_from_slice(chunk);

        loop {
            match codec.decode(&mut buffer) {
                Ok(Some(frame)) => frames.push(frame),
                Ok(None) => break,
                Err(_) => return (frames, true),
            }
        }
    }

    (frames, false)
}
