use std::time::Instant;

use crate::Resource;
use crate::terminal::{Blocks, Scanner};

pub struct Seen {
    pub marks: Vec<(u64, u64, String)>,
    pub blocks: Vec<(u64, u64)>,
}

pub fn scan(chunks: &[&[u8]]) -> Seen {
    let mut scanner = Scanner::default();
    let mut blocks = Blocks::default();
    let mut seen = Seen {
        marks: Vec::new(),
        blocks: Vec::new(),
    };
    let mut offset = 0;

    for chunk in chunks {
        for sighting in scanner.scan(offset, chunk) {
            seen.marks
                .push((sighting.start, sighting.end, format!("{:?}", sighting.mark)));

            if let Some(block) = blocks.see(sighting, Instant::now()) {
                seen.blocks.push((block.output_start, block.output_end));
            }
        }

        offset += u64::try_from(chunk.len()).unwrap_or(u64::MAX);
    }

    seen
}

pub fn read_resource(text: &str) -> Result<Resource, String> {
    Resource::from_toml(text)
}

pub fn write_resource(resource: &Resource) -> Result<String, String> {
    resource.to_toml()
}
