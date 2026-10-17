use synt_journal::NewEvent;
use synt_proto::RpcError;

pub(crate) struct Rejection {
    pub(crate) step: NewEvent,
    pub(crate) error: RpcError,
}
