use bytes::Bytes;

pub(crate) struct Chunk {
    pub(crate) data: Bytes,
    pub(crate) items: usize,
}
