use kube::config::Kubeconfig;

use super::context_info::server_of;
use super::{ClusterPlace, KubeconfigSource};
use crate::KubeError;

const DEFAULT_NAMESPACE: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClusterSpec {
    pub kubeconfig: KubeconfigSource,
    pub context: Option<String>,
    pub namespace: Option<String>,
}

impl ClusterSpec {
    pub fn new(kubeconfig: KubeconfigSource) -> Self {
        Self {
            kubeconfig,
            context: None,
            namespace: None,
        }
    }

    #[must_use]
    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    #[must_use]
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    pub fn place(&self) -> Result<ClusterPlace, KubeError> {
        let kubeconfig = self.kubeconfig.load()?;

        self.place_in(&kubeconfig)
    }

    pub(crate) fn place_in(&self, kubeconfig: &Kubeconfig) -> Result<ClusterPlace, KubeError> {
        let context = self.context_name(kubeconfig)?;

        let named = kubeconfig
            .contexts
            .iter()
            .find(|named| named.name == context)
            .ok_or_else(|| KubeError::UnknownContext {
                context: context.clone(),
                place: self.kubeconfig.to_string(),
            })?;

        let details = named.context.as_ref().ok_or_else(|| {
            self.kubeconfig
                .broken(format!("context {context} is empty"))
        })?;

        let server = server_of(kubeconfig, &details.cluster).ok_or_else(|| {
            self.kubeconfig.broken(format!(
                "context {context} uses cluster {}, which has no server",
                details.cluster
            ))
        })?;

        let namespace = self
            .namespace
            .clone()
            .or_else(|| details.namespace.clone())
            .unwrap_or_else(|| DEFAULT_NAMESPACE.to_owned());

        Ok(ClusterPlace {
            kubeconfig: self.kubeconfig.to_string(),
            context,
            server,
            namespace,
        })
    }

    fn context_name(&self, kubeconfig: &Kubeconfig) -> Result<String, KubeError> {
        if let Some(context) = &self.context {
            return Ok(context.clone());
        }

        kubeconfig
            .current_context
            .clone()
            .filter(|context| !context.is_empty())
            .ok_or_else(|| KubeError::NoContext {
                place: self.kubeconfig.to_string(),
            })
    }
}
