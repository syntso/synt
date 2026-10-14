use std::fmt::{self, Display, Formatter};

use kube::Client;
use kube::api::{Api, DynamicObject, TypeMeta};
use kube::discovery::{ApiCapabilities, ApiResource, Scope};
use serde_json::Value;

const CORE_GROUP: &str = "";
const APPS_GROUP: &str = "apps";
const SECRET: &str = "Secret";
const NAMESPACE: &str = "Namespace";
const RESTARTABLE: [&str; 3] = ["Deployment", "StatefulSet", "DaemonSet"];

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

    pub(crate) fn discovered(resource: ApiResource, capabilities: &ApiCapabilities) -> Self {
        let namespaced = matches!(capabilities.scope, Scope::Namespaced);

        Self::new(resource, namespaced)
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
        self.is_core(SECRET)
    }

    pub fn is_namespace(&self) -> bool {
        self.is_core(NAMESPACE)
    }

    pub(crate) fn is_restartable(&self) -> bool {
        self.group() == APPS_GROUP && RESTARTABLE.contains(&self.name())
    }

    pub(crate) fn is_named(&self, name: &str, plural: &str) -> bool {
        let lowered = self.name().to_lowercase();

        lowered == name || self.plural() == name || self.plural() == plural
    }

    #[must_use]
    pub(crate) fn with_version(&self, version: &str) -> Self {
        let mut resource = self.resource.clone();
        version.clone_into(&mut resource.version);

        resource.api_version = if resource.group.is_empty() {
            version.to_owned()
        } else {
            format!("{}/{version}", resource.group)
        };

        Self::new(resource, self.namespaced)
    }

    pub(crate) fn api(&self, client: &Client, namespace: Option<&str>) -> Api<DynamicObject> {
        match namespace.filter(|_| self.namespaced) {
            Some(namespace) => Api::namespaced_with(client.clone(), namespace, &self.resource),
            None => Api::all_with(client.clone(), &self.resource),
        }
    }

    pub(crate) fn tidy(&self, mut object: DynamicObject) -> Value {
        object.types = Some(TypeMeta {
            api_version: self.api_version().to_owned(),
            kind: self.name().to_owned(),
        });
        object.metadata.managed_fields = None;

        serde_json::to_value(object).unwrap_or(Value::Null)
    }

    fn is_core(&self, name: &str) -> bool {
        self.resource.group == CORE_GROUP && self.resource.kind == name
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
