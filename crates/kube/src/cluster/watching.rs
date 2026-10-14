use super::Cluster;
use crate::{Kind, Watch, WatchRequest};

impl Cluster {
    pub fn watch(&self, kind: &Kind, request: &WatchRequest) -> Watch {
        let namespace = self.namespace_among(kind, &request.namespaces);
        let api = kind.api(self.client(), namespace.as_deref());

        Watch::new(api, kind.clone(), request.selector.clone())
    }
}
