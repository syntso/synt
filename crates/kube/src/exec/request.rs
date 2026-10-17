use std::future::Future;
use std::time::Instant;

use futures_util::future;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{AttachParams, AttachedProcess};
use schemars::JsonSchema;
use serde::Deserialize;
use synt_proto::{CappedText, ExecLimits};
use tokio::io::AsyncRead;
use tokio::time;

use super::{Captured, ExitStatus};
use crate::{Cluster, Command, ExecOutcome, KubeError, Namespace, ObjectName};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ExecRequest {
    pub pod: ObjectName,

    #[serde(default)]
    pub namespace: Option<Namespace>,

    #[serde(default)]
    pub container: Option<ObjectName>,

    pub command: Command,
}

impl ExecRequest {
    pub fn new(pod: ObjectName, command: Command) -> Self {
        Self {
            pod,
            namespace: None,
            container: None,
            command,
        }
    }

    pub async fn run(
        &self,
        cluster: &Cluster,
        limits: ExecLimits,
        stop: impl Future<Output = ()>,
    ) -> Result<ExecOutcome, KubeError> {
        let started = Instant::now();
        let capturing = self.capture(cluster, limits.output_bytes);

        let captured = bounded(limits, stop, capturing).await?;

        Ok(ExecOutcome::new(captured, started.elapsed()))
    }

    async fn capture(&self, cluster: &Cluster, limit: usize) -> Result<Captured, KubeError> {
        let mut process = self.attach(cluster).await?;
        let status = process.take_status();

        let (stdout, stderr) = future::join(
            Self::read(process.stdout(), limit),
            Self::read(process.stderr(), limit),
        )
        .await;
        let status = match status {
            Some(status) => status.await,
            None => None,
        };

        process
            .join()
            .await
            .map_err(|error| KubeError::Attach(Box::new(error)))?;

        Ok(Captured {
            status: ExitStatus::from_status(status),
            stdout,
            stderr,
        })
    }

    async fn attach(&self, cluster: &Cluster) -> Result<AttachedProcess, KubeError> {
        let pods = cluster.namespaced::<Pod>(self.namespace.as_ref().map(Namespace::as_str));
        let command = self.command.words().to_vec();

        Ok(pods
            .exec(self.pod.as_str(), command, &self.params())
            .await?)
    }

    fn params(&self) -> AttachParams {
        let params = AttachParams::default()
            .stdin(false)
            .stdout(true)
            .stderr(true);

        match &self.container {
            Some(container) => params.container(container.as_str()),
            None => params,
        }
    }

    async fn read(stream: Option<impl AsyncRead + Unpin>, limit: usize) -> CappedText {
        let text = CappedText::new(limit);

        match stream {
            Some(stream) => text.read_from(stream).await,
            None => text,
        }
    }
}

async fn bounded<T>(
    limits: ExecLimits,
    stop: impl Future<Output = ()>,
    work: impl Future<Output = Result<T, KubeError>>,
) -> Result<T, KubeError> {
    let running = time::timeout(limits.timeout, work);

    tokio::select! {
        finished = running => finished.map_err(|_| KubeError::Timeout(limits.timeout))?,
        () = stop => Err(KubeError::Stopped),
    }
}
