use k8s_openapi::api::core::v1::{Pod, Service, ServicePort};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::api::{Api, ListParams};

use super::Cluster;
use crate::forward::Relay;
use crate::{ForwardTarget, KubeError, PortForward};

const RUNNING: &str = "Running";

impl Cluster {
    pub async fn forward(
        &self,
        target: &ForwardTarget,
        namespace: Option<&str>,
        port: u16,
        local_port: Option<u16>,
    ) -> Result<PortForward, KubeError> {
        let namespace = namespace.unwrap_or(self.namespace()).to_owned();
        let pods: Api<Pod> = Api::namespaced(self.client().clone(), &namespace);

        let (pod, remote_port) = match target {
            ForwardTarget::Pod(name) => (name.clone(), port),
            ForwardTarget::Service(name) => self.service_backend(&namespace, name, port).await?,
        };

        let relay = Relay::new(pods, pod, remote_port);

        PortForward::start(target.to_string(), namespace, relay, local_port).await
    }

    async fn service_backend(
        &self,
        namespace: &str,
        name: &str,
        port: u16,
    ) -> Result<(String, u16), KubeError> {
        let services: Api<Service> = Api::namespaced(self.client().clone(), namespace);
        let spec = services.get(name).await?.spec.unwrap_or_default();

        let service_port = spec
            .ports
            .unwrap_or_default()
            .into_iter()
            .find(|candidate| u16::try_from(candidate.port).ok() == Some(port))
            .ok_or_else(|| KubeError::invalid(format!("service {name} has no port {port}")))?;

        let selector = spec.selector.unwrap_or_default();
        if selector.is_empty() {
            return Err(KubeError::invalid(format!(
                "service {name} selects no pods, so there is nothing to forward to"
            )));
        }

        let labels: Vec<String> = selector
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        let pod = self.running_pod(namespace, name, &labels.join(",")).await?;
        let remote_port = target_port(&pod, &service_port)?;

        Ok((pod.metadata.name.unwrap_or_default(), remote_port))
    }

    async fn running_pod(
        &self,
        namespace: &str,
        service: &str,
        selector: &str,
    ) -> Result<Pod, KubeError> {
        let pods: Api<Pod> = Api::namespaced(self.client().clone(), namespace);
        let listed = pods.list(&ListParams::default().labels(selector)).await?;

        listed
            .items
            .into_iter()
            .find(|pod| {
                pod.status
                    .as_ref()
                    .and_then(|status| status.phase.as_deref())
                    == Some(RUNNING)
            })
            .ok_or_else(|| KubeError::invalid(format!("service {service} has no running pod")))
    }
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
