use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileAccess {
    Read,
    ReadWrite,
}

impl FileAccess {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::ReadWrite => "read-write",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        [Self::Read, Self::ReadWrite]
            .into_iter()
            .find(|access| access.as_str() == text)
    }

    pub fn covers(self, wanted: Self) -> bool {
        self >= wanted
    }

    pub fn writes(self) -> bool {
        self == Self::ReadWrite
    }
}

#[cfg(test)]
mod tests {
    use super::FileAccess;

    #[test]
    fn read_write_covers_read_but_not_the_reverse() {
        assert!(FileAccess::ReadWrite.covers(FileAccess::Read));
        assert!(FileAccess::Read.covers(FileAccess::Read));
        assert!(!FileAccess::Read.covers(FileAccess::ReadWrite));
    }

    #[test]
    fn names_round_trip() {
        for access in [FileAccess::Read, FileAccess::ReadWrite] {
            assert_eq!(FileAccess::parse(access.as_str()), Some(access));
        }

        assert_eq!(FileAccess::parse("write"), None);
    }
}
