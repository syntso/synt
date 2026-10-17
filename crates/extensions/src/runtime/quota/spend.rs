#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spend {
    Call,
    Storage,
    Message,
    Free,
}

impl Spend {
    pub(crate) fn most(self) -> u32 {
        match self {
            Self::Call => 64,
            Self::Storage => 1000,
            Self::Message => 256,
            Self::Free => u32::MAX,
        }
    }

    pub(crate) fn what(self) -> &'static str {
        match self {
            Self::Call => "calls into the core",
            Self::Storage => "storage operations",
            Self::Message => "messages to the app",
            Self::Free => "free operations",
        }
    }
}
