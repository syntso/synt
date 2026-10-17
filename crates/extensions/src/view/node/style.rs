use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(in crate::view) enum Style {
    Title,
    Body,
    Muted,
    Mono,
    Error,
    Success,
}
