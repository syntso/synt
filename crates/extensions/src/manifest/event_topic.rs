use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventTopic {
    Journal,
    Session,
    Resource,
    Approval,
}

impl EventTopic {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Journal => "journal",
            Self::Session => "session",
            Self::Resource => "resource",
            Self::Approval => "approval",
        }
    }
}
