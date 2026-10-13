use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Requested,
    Approved,
    Denied,
    Started,
    Finished,
    Failed,
    Note,
}

impl EventKind {
    pub(crate) const fn code(self) -> u8 {
        match self {
            Self::Requested => 1,
            Self::Approved => 2,
            Self::Denied => 3,
            Self::Started => 4,
            Self::Finished => 5,
            Self::Failed => 6,
            Self::Note => 7,
        }
    }

    pub(crate) const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Requested),
            2 => Some(Self::Approved),
            3 => Some(Self::Denied),
            4 => Some(Self::Started),
            5 => Some(Self::Finished),
            6 => Some(Self::Failed),
            7 => Some(Self::Note),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EventKind;

    #[test]
    fn codes_round_trip() {
        let kinds = [
            EventKind::Requested,
            EventKind::Approved,
            EventKind::Denied,
            EventKind::Started,
            EventKind::Finished,
            EventKind::Failed,
            EventKind::Note,
        ];

        for kind in kinds {
            assert_eq!(EventKind::from_code(kind.code()), Some(kind));
        }
    }

    #[test]
    fn unknown_codes_are_refused() {
        assert_eq!(EventKind::from_code(0), None);
        assert_eq!(EventKind::from_code(200), None);
    }
}
