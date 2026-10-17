use kube::config::{Kubeconfig, NamedContext};
use serde::Serialize;

use crate::kubeconfig::servers::Servers;

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
            .and_then(|cluster| kubeconfig.server_of(cluster));

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
