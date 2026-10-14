use kube::config::{Kubeconfig, NamedContext};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContextInfo {
    pub name: String,
    pub cluster: Option<String>,
    pub server: Option<String>,
    pub user: Option<String>,
    pub namespace: Option<String>,
    pub current: bool,
}

impl ContextInfo {
    pub(crate) fn new(
        kubeconfig: &Kubeconfig,
        named: &NamedContext,
        current: Option<&str>,
    ) -> Self {
        let context = named.context.as_ref();
        let cluster = context.map(|context| context.cluster.clone());
        let server = cluster
            .as_deref()
            .and_then(|cluster| server_of(kubeconfig, cluster));

        Self {
            name: named.name.clone(),
            cluster,
            server,
            user: context.and_then(|context| context.user.clone()),
            namespace: context.and_then(|context| context.namespace.clone()),
            current: current == Some(named.name.as_str()),
        }
    }
}

pub(crate) fn server_of(kubeconfig: &Kubeconfig, cluster: &str) -> Option<String> {
    kubeconfig
        .clusters
        .iter()
        .find(|named| named.name == cluster)
        .and_then(|named| named.cluster.as_ref())
        .and_then(|cluster| cluster.server.clone())
}
