use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::Deserialize;

use super::Backend;
use crate::{Cluster, KubeError, ObjectName};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(try_from = "TargetFields")]
pub enum ForwardTarget {
    Pod(ObjectName),
    Service(ObjectName),
}

impl ForwardTarget {
    pub(super) async fn backend(
        &self,
        cluster: &Cluster,
        namespace: &str,
        port: u16,
    ) -> Result<Backend, KubeError> {
        match self {
            Self::Pod(name) => Ok(Backend::pod(name, port)),
            Self::Service(name) => Backend::of_service(cluster, namespace, name, port).await,
        }
    }
}

impl Display for ForwardTarget {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pod(name) => write!(formatter, "pod/{name}"),
            Self::Service(name) => write!(formatter, "service/{name}"),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
struct TargetFields {
    #[serde(default)]
    pod: Option<ObjectName>,

    #[serde(default)]
    service: Option<ObjectName>,
}

impl TryFrom<TargetFields> for ForwardTarget {
    type Error = KubeError;

    fn try_from(fields: TargetFields) -> Result<Self, KubeError> {
        match (fields.pod, fields.service) {
            (Some(pod), None) => Ok(Self::Pod(pod)),
            (None, Some(service)) => Ok(Self::Service(service)),
            _ => Err(KubeError::invalid(
                "name either a pod or a service to forward to",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ForwardTarget;

    fn target(fields: serde_json::Value) -> Result<ForwardTarget, serde_json::Error> {
        serde_json::from_value(fields)
    }

    #[test]
    fn a_pod_or_a_service_is_the_target() {
        assert_eq!(
            target(json!({ "pod": "api" })).unwrap().to_string(),
            "pod/api"
        );
        assert_eq!(
            target(json!({ "service": "web" })).unwrap().to_string(),
            "service/web"
        );
    }

    #[test]
    fn both_or_neither_is_refused() {
        assert!(target(json!({ "pod": "api", "service": "web" })).is_err());
        assert!(target(json!({})).is_err());
    }
}
