use vte::Perform;

use super::{Mark, Sighting};

const ESCAPE: u8 = 0x1b;
const OSC_INTRODUCER: u8 = b']';
const STRING_TERMINATOR_LENGTH: u64 = 2;

#[derive(Default)]
pub(super) struct Collector {
    at: u64,
    after_escape: bool,
    sequence_start: u64,
    sightings: Vec<Sighting>,
}

impl Collector {
    pub(super) fn step(&mut self, at: u64, byte: u8) {
        if self.after_escape && byte == OSC_INTRODUCER {
            self.sequence_start = at.saturating_sub(1);
        }

        self.at = at;
        self.after_escape = byte == ESCAPE;
    }

    pub(super) fn take(&mut self) -> Vec<Sighting> {
        std::mem::take(&mut self.sightings)
    }
}

impl Perform for Collector {
    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        let Some(mark) = Mark::parse(params) else {
            return;
        };

        let end = if bell_terminated {
            self.at + 1
        } else {
            self.at + STRING_TERMINATOR_LENGTH
        };

        self.sightings.push(Sighting {
            mark,
            start: self.sequence_start,
            end,
        });
    }
}
