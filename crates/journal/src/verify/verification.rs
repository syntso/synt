use super::Damage;
use crate::model::Seq;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verification {
    Intact { checked: u64 },
    Broken { at: Seq, damage: Damage },
    Truncated { last: Seq, head: Seq },
}

impl Verification {
    pub fn is_intact(&self) -> bool {
        matches!(self, Self::Intact { .. })
    }
}
