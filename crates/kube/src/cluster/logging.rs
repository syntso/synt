use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, LogParams};

use super::Cluster;
use crate::{KubeError, LogRequest, LogStream, LogText};

const LARGEST_LOG: usize = 1024 * 1024;

impl Cluster {
    pub async fn logs(&self, request: &LogRequest) -> Result<LogText, KubeError> {
        let params = LogParams {
            limit_bytes: Some(i64::try_from(LARGEST_LOG).unwrap_or(i64::MAX)),
            ..log_params(request, false)
        };

        let text = self.pods(request).logs(&request.pod, &params).await?;

        Ok(LogText::new(text, LARGEST_LOG))
    }

    pub async fn follow_logs(&self, request: &LogRequest) -> Result<LogStream, KubeError> {
        let params = log_params(request, true);
        let reader = self.pods(request).log_stream(&request.pod, &params).await?;

        Ok(LogStream::new(reader))
    }

    fn pods(&self, request: &LogRequest) -> Api<Pod> {
        let namespace = request.namespace.as_deref().unwrap_or(self.namespace());

        Api::namespaced(self.client().clone(), namespace)
    }
}

fn log_params(request: &LogRequest, follow: bool) -> LogParams {
    LogParams {
        container: request.container.clone(),
        follow,
        previous: request.previous,
        since_seconds: request.since_seconds.map(i64::from),
        tail_lines: request.tail_lines.map(i64::from),
        timestamps: request.timestamps,
        ..LogParams::default()
    }
}
