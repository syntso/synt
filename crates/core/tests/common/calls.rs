use synt_client::ClientError;
use synt_journal::{Event, EventKind};
use synt_proto::{ActionCall, ActionOutcome, ClientKind, Hello, RpcError};

pub fn ping() -> ActionCall {
    ActionCall::new("core.ping")
}

pub fn app_hello() -> Hello {
    Hello::new(ClientKind::App, "synt-tests")
}

pub fn rejection(result: Result<ActionOutcome, ClientError>) -> RpcError {
    match result {
        Err(ClientError::Rpc(error)) => error,
        other => panic!("expected the core to refuse the call, got {other:?}"),
    }
}

pub fn kinds(events: &[Event]) -> Vec<EventKind> {
    events.iter().map(Event::kind).collect()
}
