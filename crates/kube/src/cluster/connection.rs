use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use kube::config::KubeConfigOptions;
use kube::{Client, Config};
use tokio::sync::OnceCell;
use tokio::task;

use crate::transport::DialedConnector;
use crate::{ClusterPlace, ClusterSpec, Dialer, Kind, Kinds, KubeError, Namespaces};

pub struct Cluster {
    client: Client,
    place: ClusterPlace,
    kinds: OnceCell<Kinds>,
}

impl Cluster {
    pub async fn connect(
        spec: &ClusterSpec,
        dialer: Option<Arc<dyn Dialer>>,
    ) -> Result<Self, KubeError> {
        let (config, place) = Self::configure(spec.clone()).await?;

        let client = match dialer {
            Some(dialer) => DialedConnector::new(dialer).client(&config)?,
            None => Client::try_from(config)?,
        };

        Ok(Self {
            client,
            place,
            kinds: OnceCell::new(),
        })
    }

    pub fn place(&self) -> &ClusterPlace {
        &self.place
    }

    pub fn namespace(&self) -> &str {
        &self.place.namespace
    }

    pub async fn kinds(&self) -> Result<&Kinds, KubeError> {
        self.kinds
            .get_or_try_init(|| Kinds::discover(&self.client))
            .await
    }

    pub async fn kind(&self, text: &str) -> Result<Kind, KubeError> {
        self.kinds().await?.find(text)
    }

    pub async fn version(&self) -> Result<String, KubeError> {
        let info = self.client.apiserver_version().await?;

        Ok(info.git_version)
    }

    pub(crate) fn client(&self) -> &Client {
        &self.client
    }

    pub(crate) fn namespace_for(&self, kind: &Kind, namespace: Option<&str>) -> Option<String> {
        if !kind.is_namespaced() {
            return None;
        }

        Some(namespace.unwrap_or(self.namespace()).to_owned())
    }

    pub(crate) fn namespace_among(&self, kind: &Kind, namespaces: &Namespaces) -> Option<String> {
        if !kind.is_namespaced() {
            return None;
        }

        self.namespace_named(namespaces)
    }

    pub(crate) fn namespace_named(&self, namespaces: &Namespaces) -> Option<String> {
        match namespaces {
            Namespaces::Default => Some(self.namespace().to_owned()),
            Namespaces::Only(namespace) => Some(namespace.clone()),
            Namespaces::All => None,
        }
    }

    async fn configure(spec: ClusterSpec) -> Result<(Config, ClusterPlace), KubeError> {
        let source = spec.kubeconfig.clone();
        let read = task::spawn_blocking(move || {
            let kubeconfig = spec.kubeconfig.load()?;
            let place = spec.place_in(&kubeconfig)?;

            Ok::<_, KubeError>((kubeconfig, place))
        });
        let (kubeconfig, place) = read.await.map_err(|_| {
            KubeError::Connect("reading the kubeconfig stopped unexpectedly".to_owned())
        })??;

        let options = KubeConfigOptions {
            context: Some(place.context.clone()),
            ..KubeConfigOptions::default()
        };
        let mut config = Config::from_custom_kubeconfig(kubeconfig, &options)
            .await
            .map_err(|error| source.broken(error))?;

        config.default_namespace.clone_from(&place.namespace);

        Ok((config, place))
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
