use synt_kube::{ClusterSpec, KubeconfigSource};
use synt_ssh::Target;

use super::{
    ClusterResource, HostResource, Problem, ProjectInfo, Resolved, Resource, ResourceEntry,
    ResourceListing,
};
use crate::CoreError;

pub(crate) const PERSONAL: &str = "personal";

const NAME: &str = "Personal";
const CONFIG: &str = ".ssh/config";
const LONGEST_NAME: usize = 64;

#[derive(Debug, Clone)]
pub(crate) struct Personal {
    kubeconfig: KubeconfigSource,
}

impl Personal {
    pub(crate) fn new(kubeconfig: KubeconfigSource) -> Self {
        Self { kubeconfig }
    }

    pub(crate) fn kubeconfig(&self) -> &KubeconfigSource {
        &self.kubeconfig
    }

    pub(crate) fn info(&self) -> Result<ProjectInfo, CoreError> {
        let aliases = aliases()?;
        let (clusters, problems) = self.clusters(&aliases);

        Ok(ProjectInfo {
            id: PERSONAL.to_owned(),
            name: NAME.to_owned(),
            slug: PERSONAL.to_owned(),
            path: config_path(),
            read_only: true,
            resources: aliases.len() + clusters.len(),
            problems,
        })
    }

    pub(crate) fn listing(&self) -> Result<ResourceListing, CoreError> {
        let aliases = aliases()?;
        let (clusters, problems) = self.clusters(&aliases);

        let mut resources = aliases
            .iter()
            .map(|alias| host_entry(alias))
            .collect::<Result<Vec<_>, _>>()?;
        resources.extend(clusters);

        Ok(ResourceListing {
            project: PERSONAL.to_owned(),
            resources,
            problems,
        })
    }

    pub(crate) fn resource(&self, name: &str) -> Result<ResourceEntry, CoreError> {
        let aliases = aliases()?;

        if aliases.iter().any(|alias| alias == name) {
            return host_entry(name);
        }

        self.cluster(&aliases, name)
    }

    pub(crate) fn resolve(&self, name: &str) -> Result<Resolved, CoreError> {
        let aliases = aliases()?;

        if aliases.iter().any(|alias| alias == name) {
            return Ok(Resolved::Host(Target::resolve(name)?));
        }

        let Resource::Cluster(cluster) = self.cluster(&aliases, name)?.resource else {
            return Err(missing(name));
        };

        Ok(Resolved::Cluster {
            spec: self.spec(cluster.context),
            via: None,
        })
    }

    pub(crate) fn context(&self, context: &str) -> Resolved {
        Resolved::Cluster {
            spec: self.spec(Some(context.to_owned())),
            via: None,
        }
    }

    pub(crate) fn read_only() -> CoreError {
        CoreError::Project(format!(
            "the Personal project is read-only; edit {} or the kubeconfig instead",
            config_path()
        ))
    }

    fn spec(&self, context: Option<String>) -> ClusterSpec {
        ClusterSpec {
            kubeconfig: self.kubeconfig.clone(),
            context,
            namespace: None,
        }
    }

    fn cluster(&self, aliases: &[String], name: &str) -> Result<ResourceEntry, CoreError> {
        let (clusters, _) = self.clusters(aliases);

        clusters
            .into_iter()
            .find(|entry| entry.name == name)
            .ok_or_else(|| missing(name))
    }

    fn clusters(&self, aliases: &[String]) -> (Vec<ResourceEntry>, Vec<Problem>) {
        if !self.kubeconfig.exists() {
            return (Vec::new(), Vec::new());
        }

        let contexts = match self.kubeconfig.contexts() {
            Ok(contexts) => contexts,
            Err(error) => {
                let problem = Problem {
                    file: self.kubeconfig.to_string(),
                    error: error.to_string(),
                };
                return (Vec::new(), vec![problem]);
            }
        };

        let mut entries: Vec<ResourceEntry> = Vec::new();

        for context in contexts {
            let name = resource_name(&context.name);
            let taken = aliases.contains(&name) || entries.iter().any(|entry| entry.name == name);

            if name.is_empty() || taken {
                continue;
            }

            entries.push(cluster_entry(name, context.name));
        }

        (entries, Vec::new())
    }
}

fn aliases() -> Result<Vec<String>, CoreError> {
    Ok(synt_ssh::config_aliases()?)
}

fn host_entry(alias: &str) -> Result<ResourceEntry, CoreError> {
    let target = Target::resolve(alias)?;

    Ok(ResourceEntry {
        name: alias.to_owned(),
        resource: Resource::Host(host_resource(&target)),
    })
}

fn cluster_entry(name: String, context: String) -> ResourceEntry {
    ResourceEntry {
        name,
        resource: Resource::Cluster(ClusterResource {
            kubeconfig: None,
            context: Some(context),
            namespace: None,
            via: None,
        }),
    }
}

fn resource_name(context: &str) -> String {
    let allowed = |character: char| character.is_ascii_alphanumeric() || "-_.".contains(character);

    let name: String = context
        .chars()
        .map(|character| if allowed(character) { character } else { '-' })
        .collect();

    name.trim_start_matches('.')
        .chars()
        .take(LONGEST_NAME)
        .collect()
}

fn missing(name: &str) -> CoreError {
    CoreError::Project(format!(
        "there is no Host {name} in {} and no kubeconfig context of that name",
        config_path()
    ))
}

fn host_resource(target: &Target) -> HostResource {
    let via = target
        .jumps()
        .iter()
        .map(Target::label)
        .collect::<Vec<_>>()
        .join(",");

    HostResource {
        address: target.host().to_owned(),
        port: Some(target.port()),
        user: Some(target.user().to_owned()),
        via: (!via.is_empty()).then_some(via),
        identity: target
            .identity_files()
            .first()
            .map(|path| path.display().to_string()),
    }
}

fn config_path() -> String {
    synt_platform::home_directory().map_or_else(
        |_| format!("~/{CONFIG}"),
        |home| home.join(CONFIG).display().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::resource_name;

    #[test]
    fn a_context_name_becomes_a_resource_name() {
        assert_eq!(resource_name("kind-synt"), "kind-synt");
        assert_eq!(
            resource_name("arn:aws:eks:eu-west-1:123:cluster/orders"),
            "arn-aws-eks-eu-west-1-123-cluster-orders"
        );
        assert_eq!(resource_name("..hidden"), "hidden");
        assert_eq!(resource_name(&"x".repeat(80)).len(), 64);
    }
}
