use std::io;
use std::time::Duration;

use synt_core::{Core, Server};
use synt_platform::{AppCheck, Endpoint};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time;

const STOP_LIMIT: Duration = Duration::from_secs(5);

pub struct RunningServer {
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl RunningServer {
    pub fn spawn(endpoint: &Endpoint, core: Core) -> io::Result<Self> {
        let server = Server::bind(endpoint, core, AppCheck::for_this_core().unwrap())?;
        let (shutdown, signal) = oneshot::channel::<()>();

        let task = tokio::spawn(server.run(async move {
            let _ = signal.await;
        }));

        Ok(Self { shutdown, task })
    }

    pub async fn stop(self) {
        let _ = self.shutdown.send(());

        let stopped = time::timeout(STOP_LIMIT, self.task)
            .await
            .expect("the server stops once the shutdown future completes");

        stopped.unwrap();
    }
}
