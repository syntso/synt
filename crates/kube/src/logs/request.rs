use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, LogParams};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{Cluster, KubeError, LogStream, LogText, Namespace, ObjectName};

const LARGEST_LOG: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct LogRequest {
    pub pod: ObjectName,

    #[serde(default)]
    pub namespace: Option<Namespace>,

    #[serde(default)]
    pub container: Option<ObjectName>,

    #[serde(default, rename = "tail")]
    pub tail_lines: Option<u32>,

    #[serde(default)]
    pub since_seconds: Option<u32>,

    #[serde(default)]
    pub previous: bool,

    #[serde(default)]
    pub timestamps: bool,
}

impl LogRequest {
    pub fn new(pod: ObjectName) -> Self {
        Self {
            pod,
            namespace: None,
            container: None,
            tail_lines: None,
            since_seconds: None,
            previous: false,
            timestamps: false,
        }
    }

    pub async fn read(&self, cluster: &Cluster) -> Result<LogText, KubeError> {
        let params = LogParams {
            limit_bytes: Some(i64::try_from(LARGEST_LOG).unwrap_or(i64::MAX)),
            ..self.params(false)
        };

        let text = self.pods(cluster).logs(self.pod.as_str(), &params).await?;

        Ok(LogText::new(text, LARGEST_LOG))
    }

    pub async fn follow(&self, cluster: &Cluster) -> Result<LogStream, KubeError> {
        let params = self.params(true);
        let reader = self
            .pods(cluster)
            .log_stream(self.pod.as_str(), &params)
            .await?;

        Ok(LogStream::new(reader))
    }

    fn pods(&self, cluster: &Cluster) -> Api<Pod> {
        cluster.namespaced(self.namespace.as_ref().map(Namespace::as_str))
    }

    fn params(&self, follow: bool) -> LogParams {
        LogParams {
            container: self.container.as_ref().map(ToString::to_string),
            follow,
            previous: self.previous,
            since_seconds: self.since_seconds.map(i64::from),
            tail_lines: self.tail_lines.map(i64::from),
            timestamps: self.timestamps,
            ..LogParams::default()
        }
    }
}
