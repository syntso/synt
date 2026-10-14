use std::future::Future;
use std::time::Instant;

use futures_util::future;
use k8s_openapi::api::core::v1::Pod;
use kube::api::AttachParams;
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::time;

use super::{Captured, Collected, ExitStatus};
use crate::{Cluster, Command, ExecLimits, ExecOutput, KubeError, Namespace, ObjectName};

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
    ) -> Result<ExecOutput, KubeError> {
        let started = Instant::now();
        let capturing = self.capture(cluster, limits.output_bytes);

        let captured = bounded(limits, stop, capturing).await?;

        Ok(ExecOutput::new(captured, started.elapsed()))
    }

    async fn capture(&self, cluster: &Cluster, limit: usize) -> Result<Captured, KubeError> {
        let pods = cluster.namespaced::<Pod>(self.namespace.as_ref().map(Namespace::as_str));
        let mut process = pods
            .exec(
                self.pod.as_str(),
                self.command.words().to_vec(),
                &self.params(),
            )
            .await?;
        let status = process.take_status();

        let (stdout, stderr) = future::join(
            Collected::read(process.stdout(), limit),
            Collected::read(process.stderr(), limit),
        )
        .await;
        let status = match status {
            Some(status) => status.await,
            None => None,
        };

        process
            .join()
            .await
            .map_err(|error| KubeError::Connect(error.to_string()))?;

        Ok(Captured {
            status: ExitStatus::from_status(status),
            stdout,
            stderr,
        })
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
