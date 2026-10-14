use std::num::NonZeroU16;

use k8s_openapi::api::core::v1::Pod;
use schemars::JsonSchema;
use serde::Deserialize;

use super::Relay;
use crate::{Cluster, ForwardTarget, KubeError, Namespace, PortForward};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ForwardRequest {
    #[serde(flatten)]
    pub target: ForwardTarget,

    #[serde(default)]
    pub namespace: Option<Namespace>,

    pub port: NonZeroU16,

    #[serde(default)]
    pub local_port: Option<NonZeroU16>,
}

impl ForwardRequest {
    pub fn new(target: ForwardTarget, port: NonZeroU16) -> Self {
        Self {
            target,
            namespace: None,
            port,
            local_port: None,
        }
    }

    pub async fn open(&self, cluster: &Cluster) -> Result<PortForward, KubeError> {
        let namespace = self
            .namespace
            .as_ref()
            .map_or(cluster.namespace(), Namespace::as_str)
            .to_owned();

        let backend = self
            .target
            .backend(cluster, &namespace, self.port.get())
            .await?;
        let relay = Relay::new(cluster.namespaced::<Pod>(Some(&namespace)), backend);

        PortForward::start(self.target.to_string(), namespace, relay, self.local_port).await
    }
}
