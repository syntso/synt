use crate::chain::Link;
use crate::seq::Seq;

const SEQ_LEN: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mark {
    pub seq: Seq,
    pub link: Link,
}

impl Mark {
    pub(super) const ENCODED_LEN: usize = SEQ_LEN + Link::LEN;

    pub fn new(seq: Seq, link: Link) -> Self {
        Self { seq, link }
    }

    pub(super) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let (seq, link) = bytes.split_at_checked(SEQ_LEN)?;

        let seq = Seq::new(u64::from_be_bytes(seq.try_into().ok()?));
        let link = Link::from_slice(link)?;

        Some(Self::new(seq, link))
    }

    pub(super) fn write(&self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(&self.seq.get().to_be_bytes());
        bytes.extend_from_slice(self.link.as_bytes());
    }
}
