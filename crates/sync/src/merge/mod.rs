mod document_graph;
mod document_merger;
mod merge_heads;
mod merged;
mod toml_merge;

pub(crate) use document_graph::DocumentGraph;
pub use document_merger::DocumentMerger;
pub(crate) use merge_heads::merge_heads;
pub(crate) use merged::Merged;
#[cfg(fuzzing)]
pub(crate) use toml_merge::merge_toml;
