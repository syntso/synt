use std::fmt;

#[cfg(unix)]
type Raw = u32;

#[cfg(windows)]
type Raw = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserId(Raw);

impl UserId {
    pub(crate) fn new(raw: Raw) -> Self {
        Self(raw)
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
