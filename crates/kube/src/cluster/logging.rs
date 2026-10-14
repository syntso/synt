use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, LogParams};

use super::Cluster;
use crate::{KubeError, LogRequest, LogStream, LogText};

const LARGEST_LOG: usize = 1024 * 1024;

impl Cluster {
    pub async fn logs(&self, request: &LogRequest) -> Result<LogText, KubeError> {
        let params = LogParams {
            limit_bytes: Some(i64::try_from(LARGEST_LOG).unwrap_or(i64::MAX)),
            ..request.params(false)
        };

        let text = self.pods_of(request).logs(&request.pod, &params).await?;

        Ok(LogText::new(text, LARGEST_LOG))
    }

    pub async fn follow_logs(&self, request: &LogRequest) -> Result<LogStream, KubeError> {
        let params = request.params(true);
        let reader = self
            .pods_of(request)
            .log_stream(&request.pod, &params)
            .await?;

        Ok(LogStream::new(reader))
    }

    fn pods_of(&self, request: &LogRequest) -> Api<Pod> {
        self.pods(request.namespace.as_deref())
    }
}
