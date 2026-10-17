use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ClusterKey {
    pub(crate) destination: String,
    pub(crate) kubeconfig_modified: Option<SystemTime>,
}
