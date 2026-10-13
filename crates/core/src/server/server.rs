use std::future::Future;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use synt_platform::{AppCheck, Endpoint, LocalListener, LocalStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::{task, time};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use super::session::Session;
use crate::core::Core;

const ACCEPT_BACKOFF: Duration = Duration::from_millis(50);
const CONNECTION_LIMIT: usize = 64;
const DRAIN_LIMIT: Duration = Duration::from_secs(3);

pub struct Server {
    listener: LocalListener,
    core: Arc<Core>,
    sessions: TaskTracker,
    stop: CancellationToken,
    connections: Arc<Semaphore>,
    apps: Arc<AppCheck>,
}

impl Server {
    pub fn bind(endpoint: &Endpoint, core: Core, apps: AppCheck) -> io::Result<Self> {
        let listener = LocalListener::bind(endpoint)?;

        Ok(Self {
            listener,
            core: Arc::new(core),
            sessions: TaskTracker::new(),
            stop: CancellationToken::new(),
            connections: Arc::new(Semaphore::new(CONNECTION_LIMIT)),
            apps: Arc::new(apps),
        })
    }

    pub async fn run(mut self, shutdown: impl Future<Output = ()>) {
        tokio::select! {
            () = shutdown => {}
            () = self.accept_all() => {}
        }

        self.drain().await;
    }

    async fn accept_all(&mut self) {
        loop {
            let Ok(permit) = Arc::clone(&self.connections).acquire_owned().await else {
                return;
            };

            match self.listener.accept().await {
                Ok(stream) => self.open_session(stream, permit),
                Err(_) => time::sleep(ACCEPT_BACKOFF).await,
            }
        }
    }

    fn open_session(&self, stream: LocalStream, permit: OwnedSemaphorePermit) {
        let core = Arc::clone(&self.core);
        let tasks = self.sessions.clone();
        let stop = self.stop.child_token();
        let apps = Arc::clone(&self.apps);

        self.sessions.spawn(async move {
            Session::serve(stream, core, tasks, stop, apps).await;
            drop(permit);
        });
    }

    async fn drain(self) {
        let Self {
            listener,
            core,
            sessions,
            stop,
            ..
        } = self;

        drop(listener);
        stop.cancel();

        sessions.close();
        let _ = time::timeout(DRAIN_LIMIT, sessions.wait()).await;

        let _ = task::spawn_blocking(move || drop(core)).await;
    }
}
