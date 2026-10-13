#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum AppTrust {
    #[default]
    Unverified,
    TrustedLocally,
}
