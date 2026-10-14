use std::future::Future;
use std::time::Instant;

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

        Ok(ExecOutput::new(exit, &stdout, &stderr, started.elapsed()))
    }

    async fn run_command(
        &self,
        request: &ExecRequest,
        limit: usize,
    ) -> Result<(ExitStatus, Collected, Collected), KubeError> {
        let pods = self.pods(request.namespace.as_deref());
        let mut process = pods
            .exec(&request.pod, request.command.clone(), &request.params())
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
