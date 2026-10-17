use bytes::Bytes;
use synt_proto::SessionEnded;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    Data(Bytes),
    Ended(SessionEnded),
}
