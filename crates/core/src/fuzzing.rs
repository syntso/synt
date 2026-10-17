use std::time::Instant;

use crate::CoreError;
use crate::projects::Resource;
use crate::terminals::{BlockHistory, Scanner};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeenMark {
    pub start: u64,
    pub end: u64,
    pub mark: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeenBlock {
    pub output_start: u64,
    pub output_end: u64,
}

#[derive(Debug, Default)]
pub struct Seen {
    pub marks: Vec<SeenMark>,
    pub blocks: Vec<SeenBlock>,
}

pub fn scan(chunks: &[&[u8]]) -> Seen {
    let mut scanner = Scanner::default();
    let mut history = BlockHistory::default();
    let mut seen = Seen::default();
    let mut offset = 0;

    for chunk in chunks {
        for sighting in scanner.scan(offset, chunk) {
            seen.marks.push(SeenMark {
                start: sighting.start,
                end: sighting.end,
                mark: format!("{:?}", sighting.mark),
            });

            if let Some(block) = history.see(sighting, Instant::now()) {
                seen.blocks.push(SeenBlock {
                    output_start: block.output_start,
                    output_end: block.output_end,
                });
            }
        }

        offset += u64::try_from(chunk.len()).unwrap_or(u64::MAX);
    }

    seen
}

pub fn read_resource(text: &str) -> Result<Resource, CoreError> {
    Resource::from_toml(text)
}

pub fn write_resource(resource: &Resource) -> Result<String, CoreError> {
    resource.to_toml()
}
