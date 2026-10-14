use std::io;

use serde_json::Value;
use synt_journal::{Actor, Name};
use synt_platform::{AppCheck, Peer};
use synt_proto::{
    ClientKind, ErrorCode, Hello, Message, Method, ProtocolVersion, RequestId, RpcError, Welcome,
};

use crate::VERSION;
use crate::broker::requester;
use crate::log::log;

const UNVERIFIED_APP: &str = "unverified-app";

pub(super) type Greeted = Result<(Actor, Value), RpcError>;

pub(super) async fn answer_first(
    body: &[u8],
    peer: io::Result<Peer>,
    apps: &AppCheck,
) -> (Option<RequestId>, Greeted) {
    let (id, hello) = read_hello(body);

    let greeted = match hello {
        Ok(hello) => greet(hello, peer, apps).await,
        Err(error) => Err(error),
    };

    (id, greeted)
}

fn read_hello(body: &[u8]) -> (Option<RequestId>, Result<Hello, RpcError>) {
    let request = match Message::decode(body) {
        Ok(Message::Request(request)) => request,
        Ok(Message::Response(_) | Message::Notification(_)) => {
            let refused = RpcError::new(
                ErrorCode::HANDSHAKE_REQUIRED,
                "the first message must be a hello request",
            );
            return (None, Err(refused));
        }
        Err(error) => return (None, Err(error)),
    };

    if request.method() != Some(Method::Hello) {
        let refused = RpcError::new(
            ErrorCode::HANDSHAKE_REQUIRED,
            "the first request must be hello",
        );
        return (Some(request.id), Err(refused));
    }

    let hello = request.params().and_then(speaks_our_protocol);
    (Some(request.id), hello)
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

async fn greet(hello: Hello, peer: io::Result<Peer>, apps: &AppCheck) -> Greeted {
    let actor = match hello.client {
        ClientKind::App => app(peer, apps).await?,
        ClientKind::Agent { name } => agent(name)?,
    };

    let welcome = Welcome {
        protocol: ProtocolVersion::CURRENT,
        core: VERSION.to_owned(),
        actor: requester(&actor),
    };
    let welcome = serde_json::to_value(welcome)
        .map_err(|_| RpcError::internal("the welcome cannot be encoded"))?;

    Ok((actor, welcome))
}

async fn app(peer: io::Result<Peer>, apps: &AppCheck) -> Result<Actor, RpcError> {
    let verdict = match peer {
        Ok(peer) => apps.verify(peer).await.map_err(|error| error.to_string()),
        Err(error) => Err(format!(
            "the system cannot say which process says it is the app: {error}"
        )),
    };

    let Err(reason) = verdict else {
        return Ok(Actor::Person);
    };

    log!("{reason}; it acts as the agent {UNVERIFIED_APP} and cannot answer approvals");

    Name::new(UNVERIFIED_APP)
        .map(Actor::Agent)
        .map_err(|_| RpcError::internal("the unverified app name is invalid"))
}

fn agent(name: String) -> Result<Actor, RpcError> {
    Name::new(name)
        .map(Actor::Agent)
        .map_err(|error| RpcError::invalid_params(format!("agent name: {error}")))
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use synt_journal::Actor;
    use synt_proto::{
        ClientKind, ErrorCode, Hello, Message, Method, ProtocolVersion, Request, RequestId,
        Response,
    };

    use super::{agent, read_hello};

    fn encoded(hello: &Hello) -> Vec<u8> {
        let request = Request::new(RequestId(1), Method::Hello, hello).unwrap();
        Message::from(request).encode().unwrap().to_vec()
    }

    #[test]
    fn agents_act_under_their_own_name() {
        let actor = agent("agent-1".to_owned()).unwrap();

        assert!(matches!(actor, Actor::Agent(name) if name.as_str() == "agent-1"));
    }

    #[test]
    fn agent_names_are_validated() {
        let error = agent("evil\nname".to_owned()).unwrap_err();

        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn other_major_versions_are_refused() {
        let mut future = Hello::new(ClientKind::App, "test");
        future.protocol = ProtocolVersion { major: 2, minor: 0 };

        let (_, hello) = read_hello(&encoded(&future));

        assert_eq!(hello.unwrap_err().code, ErrorCode::PROTOCOL_MISMATCH);
    }

    #[test]
    fn anything_else_first_is_refused() {
        let call = Request::new(RequestId(1), Method::ActionCall, &json!({})).unwrap();
        let body = Message::from(call).encode().unwrap();

        let (_, hello) = read_hello(&body);

        assert_eq!(hello.unwrap_err().code, ErrorCode::HANDSHAKE_REQUIRED);
    }

    #[test]
    fn a_first_response_is_refused_without_an_id() {
        let response = Response::new(Some(RequestId(9)), Ok(json!(null)));
        let body = Message::from(response).encode().unwrap();

        let (id, hello) = read_hello(&body);

        assert_eq!(id, None);
        assert_eq!(hello.unwrap_err().code, ErrorCode::HANDSHAKE_REQUIRED);
    }

    #[test]
    fn a_hello_is_answered_under_its_own_id() {
        let (id, hello) = read_hello(&encoded(&Hello::new(ClientKind::App, "test")));

        assert_eq!(id, Some(RequestId(1)));
        assert_eq!(hello.unwrap().client, ClientKind::App);
    }
}
