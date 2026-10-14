use synt_proto::RpcError;

pub(crate) enum Refusal {
    Failed(RpcError),
    Denied(String),
}
