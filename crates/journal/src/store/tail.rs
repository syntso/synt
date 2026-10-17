use crate::seq::Seq;
use crate::timestamp::Timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tail {
    pub(crate) seq: Seq,
    pub(crate) at: Timestamp,
}
