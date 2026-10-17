use synt_extensions::{Extensions, MergeSides};
use synt_sync::{DocumentKind, DocumentMerger};

use crate::log::log;

pub struct ExtensionMerger {
    extensions: Extensions,
}

impl ExtensionMerger {
    pub fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl DocumentMerger for ExtensionMerger {
    fn merge(
        &self,
        kind: &DocumentKind,
        base: Option<&[u8]>,
        older: &[u8],
        newer: &[u8],
    ) -> Option<Vec<u8>> {
        let DocumentKind::Extension { extension, path } = kind else {
            return None;
        };
        let suffix = path.suffix();

        let merged =
            self.extensions
                .merge_document(extension, suffix, MergeSides::new(base, older, newer));

        merged
            .inspect_err(|error| {
                log!("{extension} could not merge a .{suffix} document: {error}");
            })
            .ok()
    }
}
