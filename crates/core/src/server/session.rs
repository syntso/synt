use std::io;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::StreamExt;
use synt_journal::Actor;
use synt_platform::{AppCheck, LocalStream, Peer};
use synt_proto::{Frame, FrameReader, Message, Response, split_framed};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};
use tokio::time;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use super::{Greeting, Outgoing, Responder};
use crate::broker::Registration;
use crate::core::Core;
use crate::link::Link;

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
        let writer = tokio::spawn(Outgoing::new(outgoing).write_all(queue, streams));
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

        let greeting = Greeting::answer(&body, peer, &self.apps).await;
        let (actor, reply) = match greeting.outcome {
            Ok(greeted) => (Some(greeted.actor), Ok(greeted.welcome)),
            Err(error) => (None, Err(error)),
        };

        let _ = self
            .replies
            .send(Response::new(greeting.id, reply).into())
            .await;
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
            next = self.read_message(incoming) => next,
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

        let responder = Responder {
            core: Arc::clone(&self.core),
            actor: Arc::clone(actor),
            departed: self.departed.clone(),
            link: Arc::clone(&self.link),
        };
        let replies = self.replies.clone();

        self.tasks.spawn(async move {
            let id = request.id;
            let outcome = responder.answer(request).await;

            let _ = replies.send(Response::new(Some(id), outcome).into()).await;
            drop(permit);
        });
    }

    async fn read_message(&self, incoming: &mut Incoming) -> Option<(OwnedSemaphorePermit, Bytes)> {
        loop {
            match incoming.next().await?.ok()? {
                Frame::Message(body) => {
                    let permit = Arc::clone(&self.in_flight).acquire_owned().await.ok()?;
                    return Some((permit, body));
                }
                Frame::Stream { stream, data } => self.link.deliver(stream, data),
            }
        }
    }
}
