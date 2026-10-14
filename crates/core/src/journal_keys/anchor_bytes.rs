use synt_journal::{Anchors, Link, Mark, Seq};

const VERSION: u8 = 1;
const SEQ_LENGTH: usize = 8;
const MARK_LENGTH: usize = SEQ_LENGTH + Link::LEN;
const ANCHORS_LENGTH: usize = 1 + 2 * MARK_LENGTH;

pub(super) struct AnchorBytes(Vec<u8>);

impl AnchorBytes {
    pub(super) fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub(super) fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub(super) fn anchors(&self) -> Option<Anchors> {
        let (&version, marks) = self.0.split_first()?;

        if version != VERSION || marks.len() != 2 * MARK_LENGTH {
            return None;
        }

        let (floor, head) = marks.split_at(MARK_LENGTH);

        Some(Anchors {
            floor: Self::mark(floor)?,
            head: Self::mark(head)?,
        })
    }

    fn push_mark(&mut self, mark: Mark) {
        self.0.extend_from_slice(&mark.seq.get().to_be_bytes());
        self.0.extend_from_slice(mark.link.as_bytes());
    }

    fn mark(bytes: &[u8]) -> Option<Mark> {
        let (seq, link) = bytes.split_at(SEQ_LENGTH);

        let seq = Seq::new(u64::from_be_bytes(seq.try_into().ok()?));
        let link = Link::from_bytes(link.try_into().ok()?);

        Some(Mark::new(seq, link))
    }
}

impl From<&Anchors> for AnchorBytes {
    fn from(anchors: &Anchors) -> Self {
        let mut bytes = Self(Vec::with_capacity(ANCHORS_LENGTH));

        bytes.0.push(VERSION);
        bytes.push_mark(anchors.floor);
        bytes.push_mark(anchors.head);

        bytes
    }
}
