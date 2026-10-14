use synt_extensions::Extensions;
use synt_sync::{DocumentKind, DocumentMerger};

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
        let DocumentKind::Extension { extension, suffix } = kind else {
            return None;
        };

        self.extensions
            .merge_document(extension, suffix, (base, older, newer))
            .ok()
    }
}
