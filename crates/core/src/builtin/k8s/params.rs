use schemars::JsonSchema;
use serde::Deserialize;
use synt_kube::{Cluster, ClusterObject, Kind, KindName, Namespace, ObjectName};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct ObjectParams {
    pub kind: KindName,
    pub name: ObjectName,

    #[serde(default)]
    pub namespace: Option<Namespace>,
}

impl ObjectParams {
    pub(crate) fn on(&self, cluster: &Cluster, kind: &Kind) -> ClusterObject {
        cluster.object(kind, &self.name, self.namespace.as_ref())
    }
}
