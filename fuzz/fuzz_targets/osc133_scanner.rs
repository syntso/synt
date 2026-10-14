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
    for (start, end, _) in &at_once.marks {
        assert!(start < end, "a mark covers at least one byte");
        assert!(*end <= length, "a mark ends inside the stream");
        assert!(*start >= previous_end, "marks do not overlap");
        previous_end = *end;
    }

    for (output_start, output_end) in &at_once.blocks {
        assert!(output_start <= output_end);
        assert!(*output_end <= length);
    }
});
