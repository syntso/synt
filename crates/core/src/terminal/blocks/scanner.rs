use vte::Parser;

use super::{Collector, Sighting};

#[derive(Default)]
pub(crate) struct Scanner {
    parser: Parser,
    collector: Collector,
}

impl Scanner {
    pub(crate) fn scan(&mut self, offset: u64, data: &[u8]) -> Vec<Sighting> {
        for (at, byte) in (offset..).zip(data) {
            self.collector.step(at, *byte);
            self.parser.advance(&mut self.collector, &[*byte]);
        }

        self.collector.take()
    }
}

#[cfg(test)]
mod tests {
    use super::Scanner;
    use crate::terminal::blocks::Mark;

    #[test]
    fn marks_are_found_with_their_place_in_the_stream() {
        let mut scanner = Scanner::default();
        let data = b"$ \x1b]133;C;cmdline=ls\x07out\n\x1b]133;D;0\x1b\\";

        let sightings = scanner.scan(100, data);

        assert_eq!(sightings.len(), 2);
        assert_eq!(sightings[0].start, 102);
        assert_eq!(sightings[0].end, 100 + 21);
        assert_eq!(&data[21..25], b"out\n");
        assert_eq!(sightings[1].start, 100 + 25);
        assert_eq!(sightings[1].end, 100 + u64::try_from(data.len()).unwrap());
        assert_eq!(sightings[1].mark, Mark::CommandEnd { exit: Some(0) });
    }

    #[test]
    fn a_mark_split_across_chunks_is_still_found() {
        let mut scanner = Scanner::default();

        let first = scanner.scan(0, b"\x1b]13");
        let second = scanner.scan(4, b"3;A\x07");

        assert!(first.is_empty());
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].start, 0);
        assert_eq!(second[0].end, 8);
    }

    #[test]
    fn an_endless_mark_is_cut_and_later_marks_are_still_found() {
        let mut scanner = Scanner::default();
        let filler = vec![b'a'; 4 * 1024 * 1024];

        let opened = scanner.scan(0, b"\x1b]133;C;cmdline=");
        let flooded = scanner.scan(18, &filler);
        let closed = scanner.scan(18 + 4 * 1024 * 1024, b"\x07\x1b]133;D;0\x07");

        assert!(opened.is_empty());
        assert!(flooded.is_empty());
        assert_eq!(closed.len(), 2);
        let Mark::OutputStart {
            command: Some(command),
        } = &closed[0].mark
        else {
            panic!("the cut mark is still an output start");
        };
        assert!(command.len() < 1024);
        assert_eq!(closed[1].mark, Mark::CommandEnd { exit: Some(0) });
    }
}
