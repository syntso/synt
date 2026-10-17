#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_core::fuzzing::scan;

fuzz_target!(|data: &[u8]| {
    let (whole, split) = synt_fuzz::pieces(data);
    let length = u64::try_from(whole.len()).unwrap();

    let at_once = scan(&[whole]);
    let in_pieces = scan(&split);

    assert_eq!(at_once.marks, in_pieces.marks);
    assert_eq!(at_once.blocks, in_pieces.blocks);

    let mut previous_end = 0;
    for mark in &at_once.marks {
        assert!(mark.start < mark.end, "a mark covers at least one byte");
        assert!(mark.end <= length, "a mark ends inside the stream");
        assert!(mark.start >= previous_end, "marks do not overlap");
        previous_end = mark.end;
    }

    for block in &at_once.blocks {
        assert!(block.output_start <= block.output_end);
        assert!(block.output_end <= length);
    }
});
