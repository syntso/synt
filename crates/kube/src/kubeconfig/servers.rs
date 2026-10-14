use kube::config::Kubeconfig;

pub(super) trait Servers {
    fn server_of(&self, cluster: &str) -> Option<String>;
}

impl Servers for Kubeconfig {
    fn server_of(&self, cluster: &str) -> Option<String> {
        self.clusters
            .iter()
            .find(|named| named.name == cluster)
            .and_then(|named| named.cluster.as_ref())
            .and_then(|cluster| cluster.server.clone())
    }
}
