use kube::Config;
use kube::config::{KubeConfigOptions, Kubeconfig};
use tokio::task;

use crate::{ClusterPlace, ClusterSpec, KubeError};

pub(crate) struct ChosenContext {
    kubeconfig: Kubeconfig,
    place: ClusterPlace,
}

impl ChosenContext {
    pub(crate) async fn read(spec: ClusterSpec) -> Result<Self, KubeError> {
        let reading = task::spawn_blocking(move || Self::choose(&spec));

        reading.await?
    }

    pub(crate) async fn config(&self) -> Result<Config, KubeError> {
        let options = KubeConfigOptions {
            context: Some(self.place.context.clone()),
            ..KubeConfigOptions::default()
        };

        let mut config = Config::from_custom_kubeconfig(self.kubeconfig.clone(), &options)
            .await
            .map_err(|error| KubeError::kubeconfig(&self.place.kubeconfig, error))?;
        config.default_namespace.clone_from(&self.place.namespace);

        Ok(config)
    }

    pub(crate) fn into_place(self) -> ClusterPlace {
        self.place
    }

    fn choose(spec: &ClusterSpec) -> Result<Self, KubeError> {
        let kubeconfig = spec.kubeconfig.load()?;
        let place = spec.place_in(&kubeconfig)?;

        Ok(Self { kubeconfig, place })
    }
}
