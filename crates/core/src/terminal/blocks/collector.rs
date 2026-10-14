use vte::Perform;

use super::{Mark, Sighting};

const ESCAPE: u8 = 0x1b;
const OSC_INTRODUCER: u8 = b']';
const STRING_TERMINATOR_END: u8 = b'\\';

#[derive(Default)]
pub(super) struct Collector {
    at: u64,
    byte: u8,
    escape_start: Option<u64>,
    sequence_start: u64,
    ended_by_escape: Option<Sighting>,
    sightings: Vec<Sighting>,
}

impl Collector {
    pub(super) fn step(&mut self, at: u64, byte: u8) {
        if let Some(sighting) = self.ended_by_escape.take() {
            self.close(sighting, at, byte);
        }

        self.follow_escape(at, byte);

        self.at = at;
        self.byte = byte;
    }

    pub(super) fn take(&mut self) -> Vec<Sighting> {
        std::mem::take(&mut self.sightings)
    }

    fn follow_escape(&mut self, at: u64, byte: u8) {
        self.escape_start = match (self.escape_start, byte) {
            (None, ESCAPE) => Some(at),
            (Some(start), OSC_INTRODUCER) => {
                self.sequence_start = start;
                None
            }
            (Some(start), byte) if byte == ESCAPE || leaves_escape_open(byte) => Some(start),
            _ => None,
        };
    }

    fn close(&mut self, mut sighting: Sighting, at: u64, byte: u8) {
        if byte == STRING_TERMINATOR_END {
            sighting.end = at + 1;
        }

        self.sightings.push(sighting);
    }
}

impl Perform for Collector {
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        let Some(mark) = Mark::parse(params) else {
            return;
        };

        let sighting = Sighting {
            mark,
            start: self.sequence_start,
            end: self.at + 1,
        };

        if self.byte == ESCAPE {
            self.ended_by_escape = Some(Sighting {
                end: self.at,
                ..sighting
            });
            return;
        }

        self.sightings.push(sighting);
    }
}

fn leaves_escape_open(byte: u8) -> bool {
    matches!(byte, 0x00..=0x17 | 0x19 | 0x1c..=0x1f | 0x7f..=0xff)
}
