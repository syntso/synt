#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Trusted,
    Unknown,
    Changed,
    OtherTypeKnown,
}
