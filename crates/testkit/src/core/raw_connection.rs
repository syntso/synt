use std::time::Duration;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{Value, json};
use synt_platform::{Endpoint, LocalStream};
use synt_proto::{
    Frame, FrameCodec, FrameReader, FrameWriter, Hello, Message, Method, Outcome, ProtoError,
    Request, RequestId, Response,
};
use tokio::time;

use super::app_hello;
use crate::TestError;

const ANSWER_LIMIT: Duration = Duration::from_secs(5);

pub struct RawConnection {
    incoming: FrameReader<LocalStream>,
    outgoing: FrameWriter<LocalStream>,
}

impl RawConnection {
    pub async fn open(endpoint: &Endpoint) -> Result<Self, TestError> {
        let stream = LocalStream::connect(endpoint).await?;
        let (incoming, outgoing) = FrameCodec::split(stream, FrameCodec::DEFAULT_LIMIT);

        Ok(Self { incoming, outgoing })
    }

    pub async fn greeted(endpoint: &Endpoint) -> Result<Self, TestError> {
        let mut connection = Self::open(endpoint).await?;

        connection.hello(RequestId::new(0), &app_hello()).await?;
        let welcome = connection.receive().await?;

        assert!(matches!(welcome.outcome, Outcome::Result(_)));

        Ok(connection)
    }

    pub async fn hello(&mut self, id: RequestId, hello: &Hello) -> Result<(), TestError> {
        self.request(id, Method::Hello, hello).await
    }

    pub async fn request(
        &mut self,
        id: RequestId,
        method: Method,
        params: &impl Serialize,
    ) -> Result<(), TestError> {
        let request = Request::new(id, method, params)?;
        let body = Message::from(request).encode()?;

        self.send_bytes(body).await
    }

    pub async fn request_named(
        &mut self,
        id: RequestId,
        method: &str,
        params: Value,
    ) -> Result<(), TestError> {
        let request = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let body = serde_json::to_vec(&request)?;

        self.send_bytes(body).await
    }

    pub async fn send_bytes(&mut self, body: impl Into<Bytes>) -> Result<(), TestError> {
        self.outgoing.send(Frame::message(body)).await?;

        Ok(())
    }

    pub async fn send_bytes_until_refused(&mut self, body: impl Into<Bytes>) {
        let _ = self.outgoing.send(Frame::message(body)).await;
    }

    pub async fn receive(&mut self) -> Result<Response, TestError> {
        let frame = self.next_frame().await?;

        let Some(Ok(Frame::Message(body))) = frame else {
            return Err(format!("expected a message frame, got {frame:?}").into());
        };

        match Message::decode(&body)? {
            Message::Response(response) => Ok(response),
            other => Err(format!("expected a response, got {other:?}").into()),
        }
    }

    pub async fn is_closed(&mut self) -> Result<bool, TestError> {
        let frame = self.next_frame().await?;

        Ok(!matches!(frame, Some(Ok(_))))
    }

    async fn next_frame(&mut self) -> Result<Option<Result<Frame, ProtoError>>, TestError> {
        let frame = time::timeout(ANSWER_LIMIT, self.incoming.next())
            .await
            .map_err(|_| "the core did not answer in time")?;

        Ok(frame)
    }
}
