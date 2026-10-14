use schemars::JsonSchema;
use serde::Deserialize;

use crate::{Cluster, Kind, NamespaceScope, Watch};

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, JsonSchema)]
pub struct WatchRequest {
    #[serde(flatten)]
    pub scope: NamespaceScope,

    #[serde(default)]
    pub selector: Option<String>,
}

impl WatchRequest {
    pub fn open(&self, cluster: &Cluster, kind: &Kind) -> Watch {
        let api = cluster.objects(kind, &self.scope);

        Watch::new(api, kind.clone(), self.selector.clone())
    }
}
