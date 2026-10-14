use super::Cluster;
use crate::{Kind, Namespaces, Watch, WatchRequest};

impl Cluster {
    pub fn watch(&self, kind: &Kind, request: &WatchRequest) -> Watch {
        let namespace = match (&request.namespaces, kind.is_namespaced()) {
            (_, false) | (Namespaces::All, true) => None,
            (Namespaces::Only(namespace), true) => Some(namespace.as_str()),
            (Namespaces::Default, true) => Some(self.namespace()),
        };

        let api = kind.api(self.client(), namespace);

        Watch::new(api, kind.clone(), request.selector.clone())
    }
}
