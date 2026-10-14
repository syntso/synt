use std::future::Future;
use std::time::Instant;

use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, AttachParams};
use tokio::time;

use super::Cluster;
use crate::exec::{Collected, ExitStatus};
use crate::{ExecLimits, ExecOutput, ExecRequest, KubeError};

impl Cluster {
    pub async fn exec(
        &self,
        request: &ExecRequest,
        limits: ExecLimits,
        stop: impl Future<Output = ()>,
    ) -> Result<ExecOutput, KubeError> {
        if request.command.is_empty() {
            return Err(KubeError::invalid("the command is empty"));
        }

        let started = Instant::now();
        let running = time::timeout(
            limits.timeout,
            self.run_command(request, limits.output_bytes),
        );

        let (exit, stdout, stderr) = tokio::select! {
            finished = running => finished.map_err(|_| KubeError::Timeout(limits.timeout))??,
            () = stop => return Err(KubeError::Stopped),
        };

        let (exit, failure) = match exit {
            ExitStatus::Exited(code) => (Some(code), None),
            ExitStatus::Failed(reason) => (None, Some(reason)),
        };

        Ok(ExecOutput {
            exit,
            failure,
            stdout: stdout.text(),
            stderr: stderr.text(),
            truncated: stdout.truncated || stderr.truncated,
            duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        })
    }

    async fn run_command(
        &self,
        request: &ExecRequest,
        limit: usize,
    ) -> Result<(ExitStatus, Collected, Collected), KubeError> {
        let namespace = request.namespace.as_deref().unwrap_or(self.namespace());
        let pods: Api<Pod> = Api::namespaced(self.client().clone(), namespace);

        let mut params = AttachParams::default()
            .stdin(false)
            .stdout(true)
            .stderr(true);
        if let Some(container) = &request.container {
            params = params.container(container);
        }

        let mut process = pods
            .exec(&request.pod, request.command.clone(), &params)
            .await?;
        let status = process.take_status();

        let (stdout, stderr) = tokio::join!(
            Collected::read(process.stdout(), limit),
            Collected::read(process.stderr(), limit)
        );
        let status = match status {
            Some(status) => status.await,
            None => None,
        };
        process
            .join()
            .await
            .map_err(|error| KubeError::Connect(error.to_string()))?;

        Ok((ExitStatus::from_status(status), stdout, stderr))
    }
}
