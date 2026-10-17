use std::collections::VecDeque;
use std::time::Instant;

use super::{Block, Mark, OpenBlock, Sighting};
use crate::clock::milliseconds;

const KEPT_BLOCKS: usize = 1000;

#[derive(Default)]
pub(crate) struct BlockHistory {
    open: Option<OpenBlock>,
    finished: VecDeque<Block>,
    count: u64,
}

impl BlockHistory {
    pub(crate) fn see(&mut self, sighting: Sighting, now: Instant) -> Option<Block> {
        match sighting.mark {
            Mark::OutputStart { command } => {
                self.start(command, sighting.end, now);
                None
            }
            Mark::CommandEnd { exit } => self.finish(exit, sighting.start, now),
            Mark::PromptStart => self.finish(None, sighting.start, now),
            Mark::CommandStart => None,
        }
    }

    pub(crate) fn list(&self) -> Vec<Block> {
        self.finished.iter().cloned().collect()
    }

    pub(crate) fn count(&self) -> u64 {
        self.count
    }

    fn start(&mut self, command: Option<String>, output_start: u64, now: Instant) {
        if let Some(open) = self.open.as_mut() {
            open.command = open.command.take().or(command);
            return;
        }

        self.open = Some(OpenBlock {
            command,
            started: now,
            output_start,
        });
    }

    fn finish(&mut self, exit: Option<i32>, output_end: u64, now: Instant) -> Option<Block> {
        let open = self.open.take()?;

        self.count += 1;

        let block = Block {
            number: self.count,
            command: open.command,
            exit,
            duration_ms: milliseconds(now.saturating_duration_since(open.started)),
            output_start: open.output_start,
            output_end: output_end.max(open.output_start),
        };

        self.finished.push_back(block.clone());
        if self.finished.len() > KEPT_BLOCKS {
            self.finished.pop_front();
        }

        Some(block)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::BlockHistory;
    use crate::terminals::block::{Mark, Sighting};

    fn sighting(mark: Mark, start: u64, end: u64) -> Sighting {
        Sighting { mark, start, end }
    }

    #[test]
    fn output_start_and_end_make_a_block() {
        let mut blocks = BlockHistory::default();
        let started = Instant::now();
        let command = Some("false".to_owned());

        let opened = blocks.see(sighting(Mark::OutputStart { command }, 10, 20), started);
        let block = blocks
            .see(
                sighting(Mark::CommandEnd { exit: Some(1) }, 30, 40),
                started + Duration::from_millis(15),
            )
            .unwrap();

        assert!(opened.is_none());
        assert_eq!(block.number, 1);
        assert_eq!(block.command.as_deref(), Some("false"));
        assert_eq!(block.exit, Some(1));
        assert_eq!(block.duration_ms, 15);
        assert_eq!((block.output_start, block.output_end), (20, 30));
        assert_eq!(blocks.count(), 1);
        assert_eq!(blocks.list(), vec![block]);
    }

    #[test]
    fn an_end_without_a_start_is_no_block() {
        let mut blocks = BlockHistory::default();

        let block = blocks.see(
            sighting(Mark::CommandEnd { exit: Some(0) }, 0, 5),
            Instant::now(),
        );

        assert!(block.is_none());
        assert_eq!(blocks.count(), 0);
    }

    #[test]
    fn a_new_prompt_ends_a_block_the_shell_never_closed() {
        let mut blocks = BlockHistory::default();
        let now = Instant::now();

        blocks.see(sighting(Mark::OutputStart { command: None }, 0, 5), now);
        let block = blocks.see(sighting(Mark::PromptStart, 9, 14), now).unwrap();

        assert_eq!(block.exit, None);
        assert_eq!((block.output_start, block.output_end), (5, 9));
    }

    #[test]
    fn a_repeated_start_keeps_the_first_and_fills_the_command() {
        let mut blocks = BlockHistory::default();
        let now = Instant::now();
        let command = Some("ls".to_owned());

        blocks.see(sighting(Mark::OutputStart { command: None }, 0, 5), now);
        blocks.see(sighting(Mark::OutputStart { command }, 5, 10), now);
        let block = blocks
            .see(sighting(Mark::CommandEnd { exit: Some(0) }, 12, 20), now)
            .unwrap();

        assert_eq!(block.command.as_deref(), Some("ls"));
        assert_eq!(block.output_start, 5);
    }
}
