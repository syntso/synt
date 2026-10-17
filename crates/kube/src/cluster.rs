mod object;

pub use object::ClusterObject;

use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use k8s_openapi::NamespaceResourceScope;
use kube::api::{Api, DynamicObject, Resource};
use kube::{Client, Config};
use serde_json::Value;
use tokio::sync::OnceCell;

use crate::kubeconfig::ChosenContext;
use crate::objects::Fields;
use crate::transport::Connector;
use crate::{
    ClusterPlace, ClusterSpec, Dialer, Kind, KindCatalog, KindName, KubeError, Namespace,
    NamespaceScope, ObjectName, ObjectRef,
};

pub struct Cluster {
    client: Client,
    place: ClusterPlace,
    kinds: OnceCell<KindCatalog>,
}

impl Cluster {
    pub async fn connect(
        spec: &ClusterSpec,
        dialer: Option<Arc<dyn Dialer>>,
    ) -> Result<Self, KubeError> {
        let chosen = ChosenContext::read(spec.clone()).await?;
        let config = chosen.config().await?;

        Ok(Self {
            client: Self::client_for(config, dialer)?,
            place: chosen.into_place(),
            kinds: OnceCell::new(),
        })
    }

    pub fn place(&self) -> &ClusterPlace {
        &self.place
    }

    pub fn namespace(&self) -> &str {
        &self.place.namespace
    }

    pub async fn kinds(&self) -> Result<&KindCatalog, KubeError> {
        self.kinds
            .get_or_try_init(|| KindCatalog::discover(&self.client))
            .await
    }

    pub async fn kind(&self, name: &KindName) -> Result<Kind, KubeError> {
        self.kinds().await?.find(name)
    }

    pub async fn version(&self) -> Result<String, KubeError> {
        let info = self.client.apiserver_version().await?;

        Ok(info.git_version)
    }

    pub fn object(
        &self,
        kind: &Kind,
        name: &ObjectName,
        namespace: Option<&Namespace>,
    ) -> ClusterObject {
        let reference = ObjectRef {
            kind: kind.name().to_owned(),
            name: name.to_string(),
            namespace: self.namespace_for(kind, namespace.map(Namespace::as_str)),
        };

        self.placed(kind.clone(), reference)
    }

    pub(crate) async fn object_in_manifest(
        &self,
        object: &Value,
    ) -> Result<ClusterObject, KubeError> {
        let api_version = object.text("apiVersion").unwrap_or_default();
        let kind_name = object.text("kind").unwrap_or_default();
        let kind = self.kinds().await?.by_api_version(api_version, kind_name)?;

        let reference = ObjectRef {
            namespace: self.namespace_for(&kind, object.metadata_text("namespace")),
            ..ObjectRef::of(object)
        };

        Ok(self.placed(kind, reference))
    }

    pub(crate) fn client(&self) -> &Client {
        &self.client
    }

    pub(crate) fn objects(&self, kind: &Kind, scope: &NamespaceScope) -> Api<DynamicObject> {
        let namespace = kind
            .is_namespaced()
            .then(|| scope.resolve(self.namespace()))
            .flatten();

        kind.api(&self.client, namespace.as_deref())
    }

    pub(crate) fn namespaced<K>(&self, namespace: Option<&str>) -> Api<K>
    where
        K: Resource<Scope = NamespaceResourceScope>,
        K::DynamicType: Default,
    {
        let namespace = namespace.unwrap_or(self.namespace());

        Api::namespaced(self.client.clone(), namespace)
    }

    fn client_for(config: Config, dialer: Option<Arc<dyn Dialer>>) -> Result<Client, KubeError> {
        match dialer {
            Some(dialer) => Connector::new(dialer).client(&config),
            None => Ok(Client::try_from(config)?),
        }
    }

    fn placed(&self, kind: Kind, reference: ObjectRef) -> ClusterObject {
        let api = kind.api(&self.client, reference.namespace.as_deref());

        ClusterObject::new(kind, reference, api)
    }

    fn namespace_for(&self, kind: &Kind, namespace: Option<&str>) -> Option<String> {
        if !kind.is_namespaced() {
            return None;
        }

        Some(namespace.unwrap_or(self.namespace()).to_owned())
    }
}

impl Debug for Cluster {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Cluster")
            .field("place", &self.place)
            .finish_non_exhaustive()
    }
}
