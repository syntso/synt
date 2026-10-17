use bytes::Bytes;
use tokio::sync::OwnedSemaphorePermit;

pub(super) struct InFlight {
    pub(super) body: Bytes,
    pub(super) permit: OwnedSemaphorePermit,
}
