use serde_json::Value;
use synt_journal::{Actor, Name};
use synt_proto::{
    ClientKind, ErrorCode, Hello, Message, Method, ProtocolVersion, Request, RequestId, RpcError,
    Welcome,
};

use super::AppTrust;
use crate::VERSION;

const UNVERIFIED_APP: &str = "app";

pub(super) type Greeted = Result<(Actor, Value), RpcError>;

pub(super) fn answer_first(body: &[u8], trust: AppTrust) -> (Option<RequestId>, Greeted) {
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

    let greeted = greet(&request, trust).and_then(|(actor, welcome)| {
        let welcome = serde_json::to_value(welcome)
            .map_err(|_| RpcError::internal("the welcome cannot be encoded"))?;

        Ok((actor, welcome))
    });

    (Some(request.id), greeted)
}

fn greet(request: &Request, trust: AppTrust) -> Result<(Actor, Welcome), RpcError> {
    if request.method() != Some(Method::Hello) {
        return Err(RpcError::new(
            ErrorCode::HANDSHAKE_REQUIRED,
            "the first request must be hello",
        ));
    }

    let hello: Hello = request.params()?;

    if !ProtocolVersion::CURRENT.speaks_with(hello.protocol) {
        return Err(RpcError::new(
            ErrorCode::PROTOCOL_MISMATCH,
            format!(
                "core speaks protocol {}, the client speaks {}",
                ProtocolVersion::CURRENT,
                hello.protocol
            ),
        ));
    }

    let actor = actor_for(hello.client, trust)?;
    let welcome = Welcome {
        protocol: ProtocolVersion::CURRENT,
        core: VERSION.to_owned(),
    };

    Ok((actor, welcome))
}

fn actor_for(client: ClientKind, trust: AppTrust) -> Result<Actor, RpcError> {
    match (client, trust) {
        (ClientKind::App, AppTrust::TrustedLocally) => Ok(Actor::Person),
        (ClientKind::App, AppTrust::Unverified) => {
            let name = Name::new(UNVERIFIED_APP)
                .map_err(|_| RpcError::internal("the unverified app name is invalid"))?;

            Ok(Actor::Agent(name))
        }
        (ClientKind::Agent { name }, _) => {
            let name = Name::new(name)
                .map_err(|error| RpcError::invalid_params(format!("agent name: {error}")))?;

            Ok(Actor::Agent(name))
        }
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

    use super::{answer_first, greet};
    use crate::server::AppTrust;

    fn hello(hello: &Hello) -> Request {
        Request::new(RequestId(1), Method::Hello, hello).unwrap()
    }

    #[test]
    fn the_app_acts_for_the_person() {
        let (actor, welcome) = greet(
            &hello(&Hello::new(ClientKind::App, "test")),
            AppTrust::TrustedLocally,
        )
        .unwrap();

        assert_eq!(actor, Actor::Person);
        assert_eq!(welcome.protocol, ProtocolVersion::CURRENT);
    }

    #[test]
    fn agents_act_under_their_own_name() {
        let kind = ClientKind::Agent {
            name: "agent-1".to_owned(),
        };
        let (actor, _) = greet(&hello(&Hello::new(kind, "test")), AppTrust::Unverified).unwrap();

        assert!(matches!(actor, Actor::Agent(name) if name.as_str() == "agent-1"));
    }

    #[test]
    fn agent_names_are_validated() {
        let kind = ClientKind::Agent {
            name: "evil\nname".to_owned(),
        };
        let error = greet(&hello(&Hello::new(kind, "test")), AppTrust::Unverified).unwrap_err();

        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn other_major_versions_are_refused() {
        let mut future = Hello::new(ClientKind::App, "test");
        future.protocol = ProtocolVersion { major: 2, minor: 0 };

        assert_eq!(
            greet(&hello(&future), AppTrust::TrustedLocally)
                .unwrap_err()
                .code,
            ErrorCode::PROTOCOL_MISMATCH
        );
    }

    #[test]
    fn anything_else_first_is_refused() {
        let call = Request::new(RequestId(1), Method::ActionCall, &json!({})).unwrap();
        assert_eq!(
            greet(&call, AppTrust::TrustedLocally).unwrap_err().code,
            ErrorCode::HANDSHAKE_REQUIRED
        );
    }

    #[test]
    fn a_first_response_is_refused_without_an_id() {
        let response = Response::new(Some(RequestId(9)), Ok(json!(null)));
        let body = Message::from(response).encode().unwrap();

        let (id, greeted) = answer_first(&body, AppTrust::TrustedLocally);

        assert_eq!(id, None);
        assert_eq!(greeted.unwrap_err().code, ErrorCode::HANDSHAKE_REQUIRED);
    }

    #[test]
    fn a_hello_is_answered_under_its_own_id() {
        let request = hello(&Hello::new(ClientKind::App, "test"));
        let body = Message::from(request).encode().unwrap();

        let (id, greeted) = answer_first(&body, AppTrust::TrustedLocally);

        assert_eq!(id, Some(RequestId(1)));
        assert_eq!(greeted.unwrap().0, Actor::Person);
    }
}
