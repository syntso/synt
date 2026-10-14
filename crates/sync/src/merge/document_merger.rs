use crate::document::DocumentKind;

pub trait DocumentMerger: Send + Sync {
    fn merge(
        &self,
        kind: &DocumentKind,
        base: Option<&[u8]>,
        older: &[u8],
        newer: &[u8],
    ) -> Option<Vec<u8>>;
}
