mod greeted;

pub(super) use greeted::Greeted;

use std::io;

use synt_platform::{AppCheck, Peer};
use synt_proto::{ErrorCode, Hello, Message, Method, ProtocolVersion, RequestId, RpcError};

pub(super) struct Greeting<T> {
    pub(super) id: Option<RequestId>,
    pub(super) outcome: Result<T, RpcError>,
}

impl Greeting<Greeted> {
    pub(super) async fn answer(body: &[u8], peer: io::Result<Peer>, apps: &AppCheck) -> Self {
        let read = Greeting::read(body);

        let outcome = match read.outcome {
            Ok(hello) => Greeted::welcome(hello, peer, apps).await,
            Err(error) => Err(error),
        };

        Self {
            id: read.id,
            outcome,
        }
    }
}

impl Greeting<Hello> {
    fn read(body: &[u8]) -> Self {
        let request = match Message::decode(body) {
            Ok(Message::Request(request)) => request,
            Ok(Message::Response(_) | Message::Notification(_)) => {
                let refused = RpcError::new(
                    ErrorCode::HANDSHAKE_REQUIRED,
                    "the first message must be a hello request",
                );
                return Self::refused(None, refused);
            }
            Err(error) => return Self::refused(None, error),
        };

        if request.method() != Some(Method::Hello) {
            let refused = RpcError::new(
                ErrorCode::HANDSHAKE_REQUIRED,
                "the first request must be hello",
            );
            return Self::refused(Some(request.id), refused);
        }

        Self {
            id: Some(request.id),
            outcome: request.params().and_then(Self::speaks_our_protocol),
        }
    }

    fn refused(id: Option<RequestId>, error: RpcError) -> Self {
        Self {
            id,
            outcome: Err(error),
        }
    }

    fn speaks_our_protocol(hello: Hello) -> Result<Hello, RpcError> {
        if ProtocolVersion::CURRENT.speaks_with(hello.protocol) {
            return Ok(hello);
        }

        Err(RpcError::new(
            ErrorCode::PROTOCOL_MISMATCH,
            format!(
                "core speaks protocol {}, the client speaks {}",
                ProtocolVersion::CURRENT,
                hello.protocol
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use synt_journal::Actor;
    use synt_proto::{
        ClientKind, ErrorCode, Hello, Message, Method, ProtocolVersion, Request, RequestId,
        Response,
    };

    use super::{Greeted, Greeting};

    fn encoded(hello: &Hello) -> Vec<u8> {
        let request = Request::new(RequestId::new(1), Method::Hello, hello).unwrap();
        Message::from(request).encode().unwrap().to_vec()
    }

    #[test]
    fn agents_act_under_their_own_name() {
        let actor = Greeted::agent("agent-1".to_owned()).unwrap();

        assert!(matches!(actor, Actor::Agent(name) if name.as_str() == "agent-1"));
    }

    #[test]
    fn agent_names_are_validated() {
        let error = Greeted::agent("evil\nname".to_owned()).unwrap_err();

        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn other_major_versions_are_refused() {
        let mut future = Hello::new(ClientKind::App, "test");
        future.protocol = ProtocolVersion { major: 2, minor: 0 };

        let read = Greeting::read(&encoded(&future));

        assert_eq!(read.outcome.unwrap_err().code, ErrorCode::PROTOCOL_MISMATCH);
    }

    #[test]
    fn anything_else_first_is_refused() {
        let call = Request::new(RequestId::new(1), Method::ActionCall, &json!({})).unwrap();
        let body = Message::from(call).encode().unwrap();

        let read = Greeting::read(&body);

        assert_eq!(
            read.outcome.unwrap_err().code,
            ErrorCode::HANDSHAKE_REQUIRED
        );
    }

    #[test]
    fn a_first_response_is_refused_without_an_id() {
        let response = Response::new(Some(RequestId::new(9)), Ok(json!(null)));
        let body = Message::from(response).encode().unwrap();

        let read = Greeting::read(&body);

        assert_eq!(read.id, None);
        assert_eq!(
            read.outcome.unwrap_err().code,
            ErrorCode::HANDSHAKE_REQUIRED
        );
    }

    #[test]
    fn a_hello_is_answered_under_its_own_id() {
        let read = Greeting::read(&encoded(&Hello::new(ClientKind::App, "test")));

        assert_eq!(read.id, Some(RequestId::new(1)));
        assert_eq!(read.outcome.unwrap().client, ClientKind::App);
    }
}
