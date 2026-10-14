use std::sync::Arc;
use std::time::Duration;

use synt_core::{Core, Server};
use synt_platform::{AppCheck, Endpoint};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time;

const STOP_LIMIT: Duration = Duration::from_secs(30);

pub struct RunningCore {
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl RunningCore {
    pub fn spawn(endpoint: &Endpoint, core: Core) -> Self {
        let apps = AppCheck::for_this_core().unwrap();
        let server = Server::bind(endpoint, Arc::new(core), apps).unwrap();
        let (shutdown, signal) = oneshot::channel::<()>();

        let task = tokio::spawn(server.run(async move {
            let _ = signal.await;
        }));

        Self { shutdown, task }
    }

    pub async fn stop(self) {
        let _ = self.shutdown.send(());

        let Ok(stopped) = time::timeout(STOP_LIMIT, self.task).await else {
            panic!("the core did not stop once asked to");
        };
        stopped.unwrap();
    }
}
