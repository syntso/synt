use k8s_openapi::api::core::v1::{Pod, Service, ServicePort};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::api::{Api, ListParams};

use crate::{Cluster, KubeError, ObjectName};

const RUNNING: &str = "Running";

pub(super) struct Backend {
    pub(super) pod: String,
    pub(super) port: u16,
}

impl Backend {
    pub(super) fn pod(name: &ObjectName, port: u16) -> Self {
        Self {
            pod: name.to_string(),
            port,
        }
    }

    pub(super) async fn of_service(
        cluster: &Cluster,
        namespace: &str,
        name: &ObjectName,
        port: u16,
    ) -> Result<Self, KubeError> {
        let services: Api<Service> = cluster.namespaced(Some(namespace));
        let spec = services.get(name.as_str()).await?.spec.unwrap_or_default();

        let service_port = spec
            .ports
            .unwrap_or_default()
            .into_iter()
            .find(|candidate| u16::try_from(candidate.port).ok() == Some(port))
            .ok_or_else(|| KubeError::invalid(format!("service {name} has no port {port}")))?;

        let selector = Self::selector(name, spec.selector.unwrap_or_default())?;
        let pod = Self::running_pod(cluster, namespace, name, &selector).await?;
        let port = target_port(&pod, &service_port)?;

        Ok(Self {
            pod: pod.metadata.name.unwrap_or_default(),
            port,
        })
    }

    fn selector(
        service: &ObjectName,
        labels: impl IntoIterator<Item = (String, String)>,
    ) -> Result<String, KubeError> {
        let labels: Vec<String> = labels
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();

        if labels.is_empty() {
            return Err(KubeError::invalid(format!(
                "service {service} selects no pods, so there is nothing to forward to"
            )));
        }

        Ok(labels.join(","))
    }

    async fn running_pod(
        cluster: &Cluster,
        namespace: &str,
        service: &ObjectName,
        selector: &str,
    ) -> Result<Pod, KubeError> {
        let pods: Api<Pod> = cluster.namespaced(Some(namespace));
        let listed = pods.list(&ListParams::default().labels(selector)).await?;

        listed
            .items
            .into_iter()
            .find(is_running)
            .ok_or_else(|| KubeError::invalid(format!("service {service} has no running pod")))
    }
}

fn is_running(pod: &Pod) -> bool {
    pod.status
        .as_ref()
        .and_then(|status| status.phase.as_deref())
        == Some(RUNNING)
}

fn target_port(pod: &Pod, service_port: &ServicePort) -> Result<u16, KubeError> {
    let number = match &service_port.target_port {
        None => service_port.port,
        Some(IntOrString::Int(number)) => *number,
        Some(IntOrString::String(name)) => named_port(pod, name)?,
    };

    u16::try_from(number).map_err(|_| KubeError::invalid(format!("port {number} is out of range")))
}

fn named_port(pod: &Pod, name: &str) -> Result<i32, KubeError> {
    pod.spec
        .iter()
        .flat_map(|spec| spec.containers.iter())
        .flat_map(|container| container.ports.iter().flatten())
        .find(|port| port.name.as_deref() == Some(name))
        .map(|port| port.container_port)
        .ok_or_else(|| {
            KubeError::invalid(format!("no container of the pod has a port named {name}"))
        })
}
