use std::time::Duration;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{Value, json};
use synt_platform::{Endpoint, LocalStream};
use synt_proto::{
    Frame, FrameCodec, FrameError, FrameReader, FrameWriter, Hello, Message, Method, Outcome,
    Request, RequestId, Response, split_framed,
};
use tokio::time;

use super::app_hello;

const ANSWER_LIMIT: Duration = Duration::from_secs(5);

pub struct RawConnection {
    incoming: FrameReader<LocalStream>,
    outgoing: FrameWriter<LocalStream>,
}

impl RawConnection {
    pub async fn open(endpoint: &Endpoint) -> Self {
        let stream = LocalStream::connect(endpoint).await.unwrap();
        let (incoming, outgoing) = split_framed(stream, FrameCodec::DEFAULT_LIMIT);

        Self { incoming, outgoing }
    }

    pub async fn greeted(endpoint: &Endpoint) -> Self {
        let mut connection = Self::open(endpoint).await;

        connection.hello(RequestId(0), &app_hello()).await;
        let welcome = connection.receive().await;

        assert!(matches!(welcome.outcome, Outcome::Result(_)));
        connection
    }

    pub async fn hello(&mut self, id: RequestId, hello: &Hello) {
        self.request(id, Method::Hello, hello).await;
    }

    pub async fn request(&mut self, id: RequestId, method: Method, params: &impl Serialize) {
        let request = Request::new(id, method, params).unwrap();
        let body = Message::from(request).encode().unwrap();

        self.send_bytes(body).await;
    }

    pub async fn request_named(&mut self, id: RequestId, method: &str, params: Value) {
        let request = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let body = serde_json::to_vec(&request).unwrap();

        self.send_bytes(body).await;
    }

    pub async fn send_bytes(&mut self, body: impl Into<Bytes>) {
        self.outgoing.send(Frame::message(body)).await.unwrap();
    }

    pub async fn send_bytes_until_refused(&mut self, body: impl Into<Bytes>) {
        let _ = self.outgoing.send(Frame::message(body)).await;
    }

    pub async fn receive(&mut self) -> Response {
        let frame = self.next_frame().await;

        let Some(Ok(Frame::Message(body))) = frame else {
            panic!("expected a message frame, got {frame:?}");
        };

        match Message::decode(&body).unwrap() {
            Message::Response(response) => response,
            other => panic!("expected a response, got {other:?}"),
        }
    }

    pub async fn is_closed(&mut self) -> bool {
        let frame = self.next_frame().await;
        !matches!(frame, Some(Ok(_)))
    }

    async fn next_frame(&mut self) -> Option<Result<Frame, FrameError>> {
        let Ok(frame) = time::timeout(ANSWER_LIMIT, self.incoming.next()).await else {
            panic!("the core did not answer in time");
        };
        frame
    }
}
