use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synt_kube::{ClusterSpec, KubeconfigSource};

use super::{ResourceName, ResourceSpec};
use crate::CoreError;
use crate::projects::LocalPath;

const LONGEST_NAMESPACE: usize = 63;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClusterResource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kubeconfig: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
}

impl ClusterResource {
    pub(crate) fn of_context(context: String) -> Self {
        Self {
            kubeconfig: None,
            context: Some(context),
            namespace: None,
            via: None,
        }
    }

    pub(crate) fn spec(&self) -> Result<ClusterSpec, CoreError> {
        let source = match &self.kubeconfig {
            Some(path) => {
                let path = LocalPath::parse(path)
                    .map_err(|reason| CoreError::field("kubeconfig", reason))?;

                KubeconfigSource::File(path.into_path())
            }
            None => KubeconfigSource::Default,
        };

        Ok(ClusterSpec {
            kubeconfig: source,
            context: self.context.clone(),
            namespace: self.namespace.clone(),
        })
    }
}

impl ResourceSpec for ClusterResource {
    fn kind(&self) -> &'static str {
        "cluster"
    }

    fn check(&self) -> Result<(), CoreError> {
        if let Some(context) = &self.context
            && (context.is_empty() || context.contains(char::is_control))
        {
            return Err(CoreError::NotAContext(context.clone()));
        }

        if let Some(namespace) = &self.namespace
            && !is_namespace(namespace)
        {
            return Err(CoreError::NotANamespace {
                namespace: namespace.clone(),
                longest: LONGEST_NAMESPACE,
            });
        }

        if let Some(via) = &self.via {
            ResourceName::check(via, "via")?;
        }

        self.spec().map(drop)
    }
}

pub(crate) fn is_namespace(text: &str) -> bool {
    let allowed = |character: char| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    };

    !text.is_empty()
        && text.len() <= LONGEST_NAMESPACE
        && text.chars().all(allowed)
        && !text.starts_with('-')
        && !text.ends_with('-')
}

#[cfg(test)]
mod tests {
    use synt_kube::KubeconfigSource;

    use super::ClusterResource;
    use crate::projects::Resource;

    #[test]
    fn a_cluster_file_names_its_kubeconfig_context_namespace_and_way_in() {
        let text = "kind = \"cluster\"\nkubeconfig = \"/etc/synt/kubeconfig\"\n\
                    context = \"prod-eu\"\nnamespace = \"orders\"\nvia = \"bastion\"\n";

        let Resource::Cluster(cluster) = Resource::from_toml(text).expect("a cluster") else {
            panic!("a cluster resource");
        };
        let spec = cluster.spec().expect("a spec");

        assert_eq!(
            spec.kubeconfig,
            KubeconfigSource::File("/etc/synt/kubeconfig".into())
        );
        assert_eq!(spec.context.as_deref(), Some("prod-eu"));
        assert_eq!(spec.namespace.as_deref(), Some("orders"));
        assert_eq!(cluster.via.as_deref(), Some("bastion"));
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
        let resource = Resource::Cluster(ClusterResource::of_context("kind-synt".to_owned()));

        assert_eq!(
            resource.to_toml().expect("toml"),
            "kind = \"cluster\"\ncontext = \"kind-synt\"\n"
        );
    }
}
