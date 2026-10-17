use k8s_openapi::api::core::v1::Event;
use kube::api::{Api, ListParams};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{Cluster, EventLine, KubeError, NamespaceScope, ObjectName};

const LARGEST_PAGE: u32 = 500;

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, JsonSchema)]
pub struct EventQuery {
    #[serde(flatten)]
    pub scope: NamespaceScope,

    #[serde(default)]
    pub name: Option<ObjectName>,
}

impl EventQuery {
    pub async fn run(&self, cluster: &Cluster) -> Result<Vec<EventLine>, KubeError> {
        let found = self.api(cluster).list(&self.params()).await?;

        let mut lines: Vec<EventLine> = found.items.into_iter().map(EventLine::from).collect();
        lines.sort_by(|first, second| first.last_seen.cmp(&second.last_seen));

        Ok(lines)
    }

    fn api(&self, cluster: &Cluster) -> Api<Event> {
        match self.scope.resolve(cluster.namespace()) {
            Some(namespace) => cluster.namespaced(Some(&namespace)),
            None => Api::all(cluster.client().clone()),
        }
    }

    fn params(&self) -> ListParams {
        let params = ListParams::default().limit(LARGEST_PAGE);

        match &self.name {
            Some(name) => params.fields(&format!("involvedObject.name={name}")),
            None => params,
        }
    }
}
