use std::sync::Arc;
use std::time::Duration;

use synt_core::{Core, Server};
use synt_platform::{AppCheck, Endpoint};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time;

use crate::TestError;

const STOP_LIMIT: Duration = Duration::from_secs(30);

pub struct RunningServer {
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl RunningServer {
    pub fn spawn(endpoint: &Endpoint, core: impl Into<Arc<Core>>) -> Result<Self, TestError> {
        let apps = AppCheck::for_this_core()?;
        let server = Server::bind(endpoint, core, apps)?;
        let (shutdown, signal) = oneshot::channel::<()>();

        let task = tokio::spawn(server.run(async move {
            let _ = signal.await;
        }));

        Ok(Self { shutdown, task })
    }

    pub async fn stop(self) -> Result<(), TestError> {
        let _ = self.shutdown.send(());

        let stopped = time::timeout(STOP_LIMIT, self.task)
            .await
            .map_err(|_| "the server did not stop once asked to")?;
        stopped?;

        Ok(())
    }
}
