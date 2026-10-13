use std::sync::Arc;

use bytes::Bytes;
use serde::de::DeserializeOwned;
use serde_json::Value;
use synt_platform::{Endpoint, LocalStream};
use synt_proto::{
    ActionCall, ActionOutcome, ApprovalAnswer, ApprovalDecision, ApprovalId, Hello, Method,
    Notification, StreamId, Welcome,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::broadcast;

use crate::transport::{Connection, Duplex};
use crate::{ClientError, StreamReceiver, Transferred, transfer};

#[derive(Clone)]
pub struct Client {
    connection: Arc<Connection>,
    welcome: Arc<Welcome>,
}

impl Client {
    pub async fn connect(endpoint: &Endpoint, hello: Hello) -> Result<Self, ClientError> {
        let stream = LocalStream::connect(endpoint).await?;
        Self::start(stream, hello).await
    }

    pub fn welcome(&self) -> &Welcome {
        &self.welcome
    }

    pub async fn call(&self, call: ActionCall) -> Result<ActionOutcome, ClientError> {
        let reply = self.connection.request(Method::ActionCall, &call).await?;
        expect_shape(reply)
    }

    pub fn notifications(&self) -> broadcast::Receiver<Notification> {
        self.connection.subscribe()
    }

    pub async fn respond(
        &self,
        id: ApprovalId,
        decision: ApprovalDecision,
    ) -> Result<(), ClientError> {
        let answer = ApprovalAnswer { id, decision };
        self.connection
            .request(Method::ApprovalRespond, &answer)
            .await?;

        Ok(())
    }

    pub fn stream(&self, stream: StreamId) -> StreamReceiver {
        self.connection.claim(stream)
    }

    pub async fn send_stream(&self, stream: StreamId, data: Bytes) -> Result<(), ClientError> {
        self.connection.send_stream(stream, data).await
    }

    pub fn open_stream(&self) -> StreamReceiver {
        self.connection.open_stream()
    }

    pub async fn send_credited<R: AsyncRead + Unpin>(
        &self,
        stream: &mut StreamReceiver,
        source: R,
        size: u64,
    ) -> Result<Transferred, ClientError> {
        transfer::send(&self.connection, stream, source, size).await
    }

    pub async fn receive_credited<W: AsyncWrite + Unpin>(
        &self,
        stream: &mut StreamReceiver,
        sink: W,
    ) -> Result<Transferred, ClientError> {
        transfer::receive(&self.connection, stream, sink).await
    }

    async fn start(stream: impl Duplex + 'static, hello: Hello) -> Result<Self, ClientError> {
        let connection = Connection::open(stream);

        let reply = connection.request(Method::Hello, &hello).await?;
        let welcome: Welcome = expect_shape(reply)?;

        Ok(Self {
            connection: Arc::new(connection),
            welcome: Arc::new(welcome),
        })
    }
}

fn expect_shape<T: DeserializeOwned>(reply: Value) -> Result<T, ClientError> {
    serde_json::from_value(reply).map_err(|error| ClientError::UnexpectedReply(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures_util::{SinkExt, StreamExt};
    use serde_json::{Value, json};
    use synt_proto::{
        ActionCall, ClientKind, ErrorCode, Frame, FrameCodec, Hello, Message, Method,
        ProtocolVersion, Request, RequestId, Requester, Response, RpcError, Welcome,
    };
    use tokio::io::DuplexStream;
    use tokio::time;
    use tokio_util::codec::Framed;

    use super::Client;
    use crate::ClientError;

    const PATIENCE: Duration = Duration::from_secs(5);

    struct FakeCore {
        framed: Framed<DuplexStream, FrameCodec>,
    }

    impl FakeCore {
        fn pair() -> (DuplexStream, Self) {
            let (client, core) = tokio::io::duplex(64 * 1024);
            let framed = Framed::new(core, FrameCodec::new());

            (client, Self { framed })
        }

        async fn receive(&mut self) -> Request {
            let Some(Ok(Frame::Message(body))) = self.framed.next().await else {
                panic!("the client sent no message");
            };

            match Message::decode(&body).unwrap() {
                Message::Request(request) => request,
                other => panic!("the client sent {other:?}"),
            }
        }

        async fn reply(&mut self, id: RequestId, outcome: Result<Value, RpcError>) {
            let body = Message::from(Response::new(Some(id), outcome))
                .encode()
                .unwrap();

            self.framed.send(Frame::message(body)).await.unwrap();
        }

        async fn welcome(&mut self) {
            let hello = self.receive().await;
            assert_eq!(hello.method(), Some(Method::Hello));

            let welcome = Welcome {
                protocol: ProtocolVersion::CURRENT,
                core: "test".to_owned(),
                actor: Requester::Person,
            };
            self.reply(hello.id, Ok(serde_json::to_value(welcome).unwrap()))
                .await;
        }

        async fn closed(&mut self) -> bool {
            matches!(time::timeout(PATIENCE, self.framed.next()).await, Ok(None))
        }
    }

    fn hello() -> Hello {
        Hello::new(ClientKind::App, "test")
    }

    fn outcome(request: &Request) -> Value {
        let call: ActionCall = request.params().unwrap();
        json!({ "request": request.id.0, "value": call.action })
    }

    async fn connected() -> (Client, FakeCore) {
        let (stream, mut core) = FakeCore::pair();
        let (client, ()) = tokio::join!(Client::start(stream, hello()), core.welcome());

        (client.unwrap(), core)
    }

    #[tokio::test]
    async fn the_handshake_keeps_the_welcome() {
        let (client, _core) = connected().await;

        assert_eq!(client.welcome().core, "test");
        assert_eq!(client.welcome().protocol, ProtocolVersion::CURRENT);
    }

    #[tokio::test]
    async fn a_refused_hello_is_an_rpc_error() {
        let (stream, mut core) = FakeCore::pair();

        let refuse = async {
            let hello = core.receive().await;
            let error = RpcError::new(ErrorCode::PROTOCOL_MISMATCH, "too new");
            core.reply(hello.id, Err(error)).await;
        };
        let (refused, ()) = tokio::join!(Client::start(stream, hello()), refuse);

        assert!(matches!(
            refused,
            Err(ClientError::Rpc(error)) if error.code == ErrorCode::PROTOCOL_MISMATCH
        ));
    }

    #[tokio::test]
    async fn replies_reach_their_callers_in_any_order() {
        let (client, mut core) = connected().await;

        let answer_backwards = async {
            let first = core.receive().await;
            let second = core.receive().await;
            assert_ne!(first.id, second.id);

            core.reply(second.id, Ok(outcome(&second))).await;
            core.reply(first.id, Ok(outcome(&first))).await;
        };
        let (left, right, ()) = tokio::join!(
            client.call(ActionCall::new("left")),
            client.call(ActionCall::new("right")),
            answer_backwards,
        );

        assert_eq!(left.unwrap().value, json!("left"));
        assert_eq!(right.unwrap().value, json!("right"));
    }

    #[tokio::test]
    async fn request_ids_are_never_reused() {
        let (client, mut core) = connected().await;
        let mut seen = Vec::new();

        for _ in 0..3 {
            let answer = async {
                let request = core.receive().await;
                core.reply(request.id, Ok(outcome(&request))).await;
                request.id
            };
            let (reply, id) = tokio::join!(client.call(ActionCall::new("core.ping")), answer);

            reply.unwrap();
            assert!(!seen.contains(&id));
            seen.push(id);
        }
    }

    #[tokio::test]
    async fn action_errors_come_back_as_rpc_errors() {
        let (client, mut core) = connected().await;

        let deny = async {
            let request = core.receive().await;
            let error = RpcError::new(ErrorCode::DENIED, "read only");
            core.reply(request.id, Err(error)).await;
        };
        let (denied, ()) = tokio::join!(client.call(ActionCall::new("fs.write")), deny);

        assert!(matches!(
            denied,
            Err(ClientError::Rpc(error)) if error.code == ErrorCode::DENIED
        ));
    }

    #[tokio::test]
    async fn malformed_outcomes_are_unexpected_replies() {
        let (client, mut core) = connected().await;

        let garble = async {
            let request = core.receive().await;
            core.reply(request.id, Ok(json!("not an outcome"))).await;
        };
        let (garbled, ()) = tokio::join!(client.call(ActionCall::new("core.ping")), garble);

        assert!(matches!(garbled, Err(ClientError::UnexpectedReply(_))));
    }

    #[tokio::test]
    async fn every_call_fails_once_the_core_is_gone() {
        let (client, mut core) = connected().await;

        let vanish = async move {
            core.receive().await;
            drop(core);
        };
        let (waiting, ()) = tokio::join!(client.call(ActionCall::new("core.ping")), vanish);
        let later = client.call(ActionCall::new("core.ping")).await;

        assert!(matches!(waiting, Err(ClientError::Disconnected)));
        assert!(matches!(later, Err(ClientError::Disconnected)));
    }

    #[tokio::test]
    async fn dropping_the_last_clone_closes_the_connection() {
        let (client, mut core) = connected().await;
        let clone = client.clone();

        drop(client);

        let answer = async {
            let request = core.receive().await;
            core.reply(request.id, Ok(outcome(&request))).await;
        };
        let (reply, ()) = tokio::join!(clone.call(ActionCall::new("core.ping")), answer);
        reply.unwrap();

        drop(clone);

        assert!(core.closed().await);
    }
}
