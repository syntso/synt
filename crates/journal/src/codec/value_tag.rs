use crate::error::JournalError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ValueTag {
    Null = 0,
    False = 1,
    True = 2,
    Signed = 3,
    Unsigned = 4,
    Text = 5,
    List = 6,
    Map = 7,
}

impl ValueTag {
    pub(super) const MAX_DEPTH: usize = 32;
}

impl From<ValueTag> for u8 {
    fn from(tag: ValueTag) -> Self {
        tag as u8
    }
}

impl TryFrom<u8> for ValueTag {
    type Error = JournalError;

    fn try_from(code: u8) -> Result<Self, JournalError> {
        let tag = match code {
            0 => Self::Null,
            1 => Self::False,
            2 => Self::True,
            3 => Self::Signed,
            4 => Self::Unsigned,
            5 => Self::Text,
            6 => Self::List,
            7 => Self::Map,
            _ => return Err(JournalError::Malformed("unknown value tag")),
        };

        Ok(tag)
    }
}
