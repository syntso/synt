use crate::seq::Seq;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pruned {
    pub removed: u64,
    pub floor: Seq,
}
