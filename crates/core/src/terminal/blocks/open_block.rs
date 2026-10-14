use std::time::Instant;

pub(super) struct OpenBlock {
    pub(super) command: Option<String>,
    pub(super) started: Instant,
    pub(super) output_start: u64,
}
