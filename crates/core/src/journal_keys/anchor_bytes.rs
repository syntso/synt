use synt_journal::{Anchors, DeviceKey, Link, Mark, Seq};
use zeroize::Zeroizing;

const VERSION: u8 = 1;
const SEQ_LEN: usize = 8;
const MARK_LEN: usize = SEQ_LEN + Link::LEN;
const ANCHORS_LEN: usize = 1 + 2 * MARK_LEN;

pub(crate) fn encode_anchors(anchors: &Anchors) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(ANCHORS_LEN);

    bytes.push(VERSION);
    push_mark(&mut bytes, anchors.floor);
    push_mark(&mut bytes, anchors.head);

    bytes
}

pub(crate) fn decode_anchors(bytes: &[u8]) -> Option<Anchors> {
    let (&version, marks) = bytes.split_first()?;

    if version != VERSION || marks.len() != 2 * MARK_LEN {
        return None;
    }

    let (floor, head) = marks.split_at(MARK_LEN);

    Some(Anchors {
        floor: read_mark(floor)?,
        head: read_mark(head)?,
    })
}

pub(crate) fn decode_device_key(bytes: &[u8]) -> Option<DeviceKey> {
    let mut key = Zeroizing::new([0; 32]);

    if bytes.len() != key.len() {
        return None;
    }

    key.copy_from_slice(bytes);
    Some(DeviceKey::from_bytes(*key))
}

fn push_mark(bytes: &mut Vec<u8>, mark: Mark) {
    bytes.extend_from_slice(&mark.seq.get().to_be_bytes());
    bytes.extend_from_slice(mark.link.as_bytes());
}

fn read_mark(bytes: &[u8]) -> Option<Mark> {
    let (seq, link) = bytes.split_at(SEQ_LEN);

    let seq = Seq::new(u64::from_be_bytes(seq.try_into().ok()?));
    let link = Link::from_bytes(link.try_into().ok()?);

    Some(Mark::new(seq, link))
}
