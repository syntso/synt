use bytes::Bytes;
use serde_json::{Map, Value};

use super::{Notification, Request, Response, RpcError};

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

impl Message {
    pub fn decode(body: &[u8]) -> Result<Self, RpcError> {
        let value: Value = serde_json::from_slice(body)
            .map_err(|error| RpcError::parse_error(error.to_string()))?;

        let Value::Object(fields) = &value else {
            return Err(RpcError::invalid_request("a message must be a JSON object"));
        };

        let message = match (fields.contains_key("method"), has_id(fields)) {
            (true, true) => serde_json::from_value(value).map(Self::Request),
            (true, false) => serde_json::from_value(value).map(Self::Notification),
            (false, _) => serde_json::from_value(value).map(Self::Response),
        };

        message.map_err(|error| RpcError::invalid_request(error.to_string()))
    }

    pub fn encode(&self) -> Result<Bytes, serde_json::Error> {
        let json = match self {
            Self::Request(request) => serde_json::to_vec(request)?,
            Self::Response(response) => serde_json::to_vec(response)?,
            Self::Notification(notification) => serde_json::to_vec(notification)?,
        };

        Ok(Bytes::from(json))
    }
}

impl From<Request> for Message {
    fn from(request: Request) -> Self {
        Self::Request(request)
    }
}

impl From<Response> for Message {
    fn from(response: Response) -> Self {
        Self::Response(response)
    }
}

impl From<Notification> for Message {
    fn from(notification: Notification) -> Self {
        Self::Notification(notification)
    }
}

fn has_id(fields: &Map<String, Value>) -> bool {
    fields.get("id").is_some_and(|id| !id.is_null())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::Message;
    use crate::Method;
    use crate::rpc::{ErrorCode, Notification, Outcome, Request, RequestId, Response, RpcError};

    fn round_trip(message: &Message) -> Message {
        Message::decode(&message.encode().unwrap()).unwrap()
    }

    #[test]
    fn requests_responses_and_notifications_round_trip() {
        let request = Request::new(
            RequestId(1),
            Method::ActionCall,
            &json!({ "action": "core.ping" }),
        )
        .unwrap();
        let success = Response::new(Some(RequestId(1)), Outcome::Result(json!({ "pong": true })));
        let failure = Response::new(
            Some(RequestId(2)),
            Outcome::Error(RpcError::new(ErrorCode::DENIED, "no")),
        );
        let event = Notification::new("journal.appended", json!({ "seq": 7 }));

        for message in [request.into(), success.into(), failure.into(), event.into()] {
            assert_eq!(round_trip(&message), message);
        }
    }

    #[test]
    fn wire_format_is_json_rpc() {
        let request = Request::new(RequestId(5), Method::Hello, &json!({})).unwrap();
        let bytes = Message::from(request).encode().unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(
            value,
            json!({ "jsonrpc": "2.0", "id": 5, "method": "hello", "params": {} })
        );
    }

    #[test]
    fn garbage_is_a_parse_error() {
        let error = Message::decode(b"{not json").unwrap_err();
        assert_eq!(error.code, ErrorCode::PARSE_ERROR);
    }

    #[test]
    fn wrong_versions_and_shapes_are_invalid_requests() {
        let wrong_version = br#"{"jsonrpc":"1.0","id":1,"method":"hello"}"#;
        let not_an_object = br"[1, 2, 3]";

        assert_eq!(
            Message::decode(wrong_version).unwrap_err().code,
            ErrorCode::INVALID_REQUEST
        );
        assert_eq!(
            Message::decode(not_an_object).unwrap_err().code,
            ErrorCode::INVALID_REQUEST
        );
    }
}
