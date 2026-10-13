use crate::chain::Link;
use crate::model::Seq;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mark {
    pub seq: Seq,
    pub link: Link,
}

impl Mark {
    pub fn new(seq: Seq, link: Link) -> Self {
        Self { seq, link }
    }
}
