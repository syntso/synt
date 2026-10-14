use std::io;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::StreamExt;
use serde::Serialize;
use serde_json::{Value, json};
use synt_journal::Actor;
use synt_platform::{AppCheck, LocalStream, Peer};
use synt_proto::{
    ActionCall, ApprovalAnswer, ErrorCode, Frame, FrameReader, Message, Method, Request, Response,
    RpcError, split_framed,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};
use tokio::time;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use super::greeting;
use super::outgoing;
use crate::broker::Registration;
use crate::core::Core;
use crate::link::Link;
use crate::log::log;

const REQUEST_FRAME_LIMIT: usize = 1024 * 1024;
const OUTGOING_BACKLOG: usize = 64;
const STREAM_BACKLOG: usize = 256;
const IN_FLIGHT_LIMIT: usize = 32;
const HANDSHAKE_LIMIT: Duration = Duration::from_secs(5);

type Incoming = FrameReader<LocalStream>;

pub(super) struct Session {
    core: Arc<Core>,
    tasks: TaskTracker,
    stop: CancellationToken,
    departed: CancellationToken,
    apps: Arc<AppCheck>,
    replies: mpsc::Sender<Message>,
    in_flight: Arc<Semaphore>,
    link: Arc<Link>,
}

impl Session {
    pub(super) async fn serve(
        stream: LocalStream,
        core: Arc<Core>,
        tasks: TaskTracker,
        stop: CancellationToken,
        apps: Arc<AppCheck>,
    ) {
        let peer = stream.peer().await;

        let (mut incoming, outgoing) = split_framed(stream, REQUEST_FRAME_LIMIT);
        let (replies, queue) = mpsc::channel(OUTGOING_BACKLOG);
        let (frames, streams) = mpsc::channel(STREAM_BACKLOG);
        let writer = tokio::spawn(outgoing::write_all(outgoing, queue, streams));
        let link = Arc::new(Link::new(frames));

        let session = Self {
            core,
            tasks,
            stop,
            departed: CancellationToken::new(),
            apps,
            replies,
            in_flight: Arc::new(Semaphore::new(IN_FLIGHT_LIMIT)),
            link: Arc::clone(&link),
        };
        session.run(&mut incoming, peer).await;
        link.close();

        let _ = writer.await;
    }

    async fn run(self, incoming: &mut Incoming, peer: io::Result<Peer>) {
        let _departure = self.departed.clone().drop_guard();

        let Some(actor) = self.greet(incoming, peer).await else {
            return;
        };
        let _registration = self.attend(&actor);
        let actor = Arc::new(actor);

        while let Some((permit, body)) = self.next_message(incoming).await {
            self.handle(&body, &actor, permit).await;
        }
    }

    async fn greet(&self, incoming: &mut Incoming, peer: io::Result<Peer>) -> Option<Actor> {
        let first = tokio::select! {
            () = self.stop.cancelled() => return None,
            first = time::timeout(HANDSHAKE_LIMIT, incoming.next()) => first,
        };
        let Ok(Some(Ok(Frame::Message(body)))) = first else {
            return None;
        };

        let (id, greeted) = greeting::answer_first(&body, peer, &self.apps).await;
        let (actor, reply) = match greeted {
            Ok((actor, welcome)) => (Some(actor), Ok(welcome)),
            Err(error) => (None, Err(error)),
        };

        let _ = self.replies.send(Response::new(id, reply).into()).await;
        actor
    }

    fn attend(&self, actor: &Actor) -> Option<Registration> {
        if *actor != Actor::Person {
            return None;
        }

        let registration = self.core.broker().register(self.replies.clone());
        Some(registration)
    }

    async fn next_message(&self, incoming: &mut Incoming) -> Option<(OwnedSemaphorePermit, Bytes)> {
        tokio::select! {
            () = self.stop.cancelled() => None,
            () = self.replies.closed() => None,
            next = read_message(incoming, &self.in_flight, &self.link) => next,
        }
    }

    async fn handle(&self, body: &[u8], actor: &Arc<Actor>, permit: OwnedSemaphorePermit) {
        let request = match Message::decode(body) {
            Ok(Message::Request(request)) => request,
            Ok(Message::Response(_) | Message::Notification(_)) => return,
            Err(error) => {
                let _ = self
                    .replies
                    .send(Response::new(None, Err(error)).into())
                    .await;
                return;
            }
        };

        let core = Arc::clone(&self.core);
        let actor = Arc::clone(actor);
        let replies = self.replies.clone();
        let departed = self.departed.clone();
        let link = Arc::clone(&self.link);

        self.tasks.spawn(async move {
            let id = request.id;
            let outcome = answer(&core, &actor, request, &departed, &link).await;

            let _ = replies.send(Response::new(Some(id), outcome).into()).await;
            drop(permit);
        });
    }
}

async fn read_message(
    incoming: &mut Incoming,
    in_flight: &Arc<Semaphore>,
    link: &Link,
) -> Option<(OwnedSemaphorePermit, Bytes)> {
    loop {
        match incoming.next().await?.ok()? {
            Frame::Message(body) => {
                let permit = Arc::clone(in_flight).acquire_owned().await.ok()?;
                return Some((permit, body));
            }
            Frame::Stream { stream, data } => link.deliver(stream, data),
        }
    }
}

async fn answer(
    core: &Core,
    actor: &Actor,
    request: Request,
    departed: &CancellationToken,
    link: &Arc<Link>,
) -> Result<Value, RpcError> {
    match request.method() {
        Some(Method::ActionCall) => {
            let call: ActionCall = request.into_params()?;
            let outcome = core.dispatch(actor, call, departed, link).await?;

            encoded(&outcome, "outcome", "the outcome cannot be encoded")
        }
        Some(Method::ActionsDescribe) => {
            let descriptions = core.describe(actor);

            encoded(
                &descriptions,
                "descriptions",
                "the action descriptions cannot be encoded",
            )
        }
        Some(Method::ApprovalRespond) => {
            let answer: ApprovalAnswer = request.into_params()?;
            core.respond(actor, answer)?;

            Ok(json!({}))
        }
        Some(Method::Hello) => Err(RpcError::invalid_request("hello was already received")),
        None => Err(RpcError::new(
            ErrorCode::METHOD_NOT_FOUND,
            format!("no method named {}", request.method),
        )),
    }
}

fn encoded(value: &impl Serialize, what: &str, refusal: &str) -> Result<Value, RpcError> {
    serde_json::to_value(value).map_err(|error| {
        log!("{what}: {error}");
        RpcError::internal(refusal)
    })
}
