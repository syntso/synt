use std::fmt::{self, Display, Formatter};

use kube::Client;
use kube::api::{Api, DynamicObject};
use kube::discovery::ApiResource;

const CORE_GROUP: &str = "";
const SECRET: &str = "Secret";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kind {
    resource: ApiResource,
    namespaced: bool,
}

impl Kind {
    pub(crate) fn new(resource: ApiResource, namespaced: bool) -> Self {
        Self {
            resource,
            namespaced,
        }
    }

    pub fn name(&self) -> &str {
        &self.resource.kind
    }

    pub fn group(&self) -> &str {
        &self.resource.group
    }

    pub fn plural(&self) -> &str {
        &self.resource.plural
    }

    pub fn api_version(&self) -> &str {
        &self.resource.api_version
    }

    pub fn is_namespaced(&self) -> bool {
        self.namespaced
    }

    pub fn is_secret(&self) -> bool {
        self.resource.group == CORE_GROUP && self.resource.kind == SECRET
    }

    pub(crate) fn resource(&self) -> &ApiResource {
        &self.resource
    }

    pub(crate) fn api(&self, client: &Client, namespace: Option<&str>) -> Api<DynamicObject> {
        match namespace.filter(|_| self.namespaced) {
            Some(namespace) => Api::namespaced_with(client.clone(), namespace, &self.resource),
            None => Api::all_with(client.clone(), &self.resource),
        }
    }
}

impl Display for Kind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if self.resource.group == CORE_GROUP {
            return formatter.write_str(&self.resource.kind);
        }

        write!(formatter, "{}.{}", self.resource.kind, self.resource.group)
    }
}
