use thiserror::Error;

use crate::model::Name;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum NameError {
    #[error("name is empty")]
    Empty,

    #[error("name is {len} bytes, the limit is {}", Name::MAX_LEN)]
    TooLong { len: usize },

    #[error("name contains a control character")]
    ControlCharacter,
}
