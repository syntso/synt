use synt_client::ClientError;
use synt_proto::{ActionOutcome, RpcError};

pub fn refusal(result: Result<ActionOutcome, ClientError>) -> RpcError {
    match result {
        Err(ClientError::Rpc(error)) => error,
        other => panic!("expected the core to refuse the call, got {other:?}"),
    }
}
