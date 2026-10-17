use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synt_kube::{ClusterSpec, KubeconfigSource, Namespace};

use super::{ContextName, ResourceName};
use crate::CoreError;
use crate::projects::LocalPath;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClusterResource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) kubeconfig: Option<LocalPath>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) context: Option<ContextName>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub(crate) namespace: Option<Namespace>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) via: Option<ResourceName>,
}

impl ClusterResource {
    pub(crate) fn of_context(context: &str) -> Result<Self, CoreError> {
        Ok(Self {
            kubeconfig: None,
            context: Some(ContextName::parse(context)?),
            namespace: None,
            via: None,
        })
    }

    pub(crate) fn spec(&self) -> Result<ClusterSpec, CoreError> {
        let source = match &self.kubeconfig {
            Some(path) => KubeconfigSource::File(path.path()?),
            None => KubeconfigSource::Default,
        };

        Ok(ClusterSpec {
            kubeconfig: source,
            context: self
                .context
                .as_ref()
                .map(|context| context.as_str().to_owned()),
            namespace: self
                .namespace
                .as_ref()
                .map(|namespace| namespace.as_str().to_owned()),
        })
    }
}

#[cfg(test)]
mod tests {
    use synt_kube::KubeconfigSource;

    use super::ClusterResource;
    use crate::projects::Resource;
    use crate::projects::local_path::rooted;

    #[test]
    fn a_cluster_file_names_its_kubeconfig_context_namespace_and_way_in() {
        let kubeconfig = rooted("/etc/synt/kubeconfig");
        let text = format!(
            "kind = \"cluster\"\nkubeconfig = \"{kubeconfig}\"\n\
             context = \"prod-eu\"\nnamespace = \"orders\"\nvia = \"bastion\"\n"
        );

        let Resource::Cluster(cluster) = Resource::from_toml(&text).expect("a cluster") else {
            panic!("a cluster resource");
        };
        let spec = cluster.spec().expect("a spec");

        assert_eq!(spec.kubeconfig, KubeconfigSource::File(kubeconfig.into()));
        assert_eq!(spec.context.as_deref(), Some("prod-eu"));
        assert_eq!(spec.namespace.as_deref(), Some("orders"));
        assert_eq!(cluster.via.map(String::from).as_deref(), Some("bastion"));
    }

    #[test]
    fn an_empty_cluster_uses_the_default_kubeconfig_and_its_current_context() {
        let Resource::Cluster(cluster) =
            Resource::from_toml("kind = \"cluster\"\n").expect("a cluster")
        else {
            panic!("a cluster resource");
        };

        let spec = cluster.spec().expect("a spec");

        assert_eq!(spec.kubeconfig, KubeconfigSource::Default);
        assert_eq!(spec.context, None);
    }

    #[test]
    fn bad_fields_are_refused_with_the_reason() {
        let cluster = |field: &str| Resource::from_toml(&format!("kind = \"cluster\"\n{field}\n"));

        assert!(cluster("kubeconfig = \"relative/config\"").is_err());
        assert!(cluster("namespace = \"Orders\"").is_err());
        assert!(cluster("namespace = \"-orders\"").is_err());
        assert!(cluster("via = \"../bastion\"").is_err());
        assert!(cluster("context = \"\"").is_err());
        assert!(cluster("server = \"https://example.com\"").is_err());
    }

    #[test]
    fn a_cluster_writes_back_only_what_it_holds() {
        let resource =
            Resource::Cluster(ClusterResource::of_context("kind-synt").expect("a context"));

        assert_eq!(
            resource.to_toml().expect("toml"),
            "kind = \"cluster\"\ncontext = \"kind-synt\"\n"
        );
    }
}
