mod clusters;

use clusters::Clusters;

use synt_kube::{ClusterSpec, KubeconfigSource};
use synt_ssh::Target;
use synt_sync::{ProjectFolder, ProjectSlug};

use super::{
    ClusterResource, HostResource, Problem, ProjectInfo, ProjectSource, Resolved, ResolvedDatabase,
    Resource, ResourceEntry, ResourceListing, ResourceName,
};
use crate::CoreError;

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

    pub(crate) fn context(&self, context: &str) -> Resolved {
        Resolved::Cluster {
            spec: self.spec(Some(context.to_owned())),
            via: None,
        }
    }

    fn spec(&self, context: Option<String>) -> ClusterSpec {
        ClusterSpec {
            kubeconfig: self.kubeconfig.clone(),
            context,
            namespace: None,
        }
    }

    fn aliases() -> Result<Vec<String>, CoreError> {
        Ok(synt_ssh::UserConfig::load()?.aliases())
    }

    fn host(alias: &str) -> Result<ResourceEntry, CoreError> {
        let target = Target::resolve(alias)?;
        let host = HostResource::of_target(&target)?;

        Ok(ResourceEntry::new(alias, Resource::Host(host)))
    }

    fn cluster(&self, aliases: &[String], name: &str) -> Result<ResourceEntry, CoreError> {
        self.clusters(aliases)
            .entries
            .into_iter()
            .find(|entry| entry.name == name)
            .ok_or_else(|| Self::missing(name))
    }

    fn clusters(&self, aliases: &[String]) -> Clusters {
        let mut clusters = Clusters::default();

        if !self.kubeconfig.exists() {
            return clusters;
        }

        let contexts = match self.kubeconfig.contexts() {
            Ok(contexts) => contexts,
            Err(error) => {
                clusters.problems.push(Problem {
                    file: self.kubeconfig.to_string(),
                    error: error.to_string(),
                });
                return clusters;
            }
        };

        for context in contexts {
            let name = Self::resource_name(&context.name);
            let taken =
                aliases.contains(&name) || clusters.entries.iter().any(|entry| entry.name == name);

            if name.is_empty() || taken {
                continue;
            }

            let Ok(cluster) = ClusterResource::of_context(&context.name) else {
                continue;
            };
            clusters
                .entries
                .push(ResourceEntry::new(&name, Resource::Cluster(cluster)));
        }

        clusters
    }

    fn resource_name(context: &str) -> String {
        let allowed =
            |character: char| character.is_ascii_alphanumeric() || "-_.".contains(character);

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
        CoreError::NoPersonalResource {
            name: name.to_owned(),
            config: Self::config_path(),
        }
    }

    fn config_path() -> String {
        synt_platform::home_directory().map_or_else(
            |_| format!("~/{CONFIG}"),
            |home| home.join(CONFIG).display().to_string(),
        )
    }
}

impl ProjectSource for Personal {
    fn info(&self) -> Result<ProjectInfo, CoreError> {
        let aliases = Self::aliases()?;
        let clusters = self.clusters(&aliases);

        Ok(ProjectInfo {
            id: ProjectSlug::PERSONAL.to_owned(),
            name: NAME.to_owned(),
            slug: ProjectSlug::PERSONAL.to_owned(),
            path: Self::config_path(),
            read_only: true,
            resources: aliases.len() + clusters.entries.len(),
            problems: clusters.problems,
        })
    }

    fn listing(&self) -> Result<ResourceListing, CoreError> {
        let aliases = Self::aliases()?;
        let clusters = self.clusters(&aliases);

        let mut resources = aliases
            .iter()
            .map(|alias| Self::host(alias))
            .collect::<Result<Vec<_>, _>>()?;
        resources.extend(clusters.entries);

        Ok(ResourceListing {
            project: ProjectSlug::PERSONAL.to_owned(),
            resources,
            problems: clusters.problems,
        })
    }

    fn resource(&self, name: &ResourceName) -> Result<ResourceEntry, CoreError> {
        let name = name.as_str();
        let aliases = Self::aliases()?;

        if aliases.iter().any(|alias| alias == name) {
            return Self::host(name);
        }

        self.cluster(&aliases, name)
    }

    fn resolve(&self, name: &ResourceName) -> Result<Resolved, CoreError> {
        let name = name.as_str();
        let aliases = Self::aliases()?;

        if aliases.iter().any(|alias| alias == name) {
            return Ok(Resolved::Host(Target::resolve(name)?));
        }

        let Resource::Cluster(cluster) = self.cluster(&aliases, name)?.resource else {
            return Err(Self::missing(name));
        };

        Ok(Resolved::Cluster {
            spec: self.spec(cluster.context.map(String::from)),
            via: None,
        })
    }

    fn database(&self, _name: &ResourceName) -> Result<ResolvedDatabase, CoreError> {
        Err(CoreError::PersonalHasNoDatabases)
    }

    fn put(
        &mut self,
        _name: &ResourceName,
        _admitted: Option<&str>,
        _resource: Resource,
    ) -> Result<ResourceEntry, CoreError> {
        Err(CoreError::PersonalReadOnly(Self::config_path()))
    }

    fn remove(
        &mut self,
        _name: &ResourceName,
        _admitted: Option<&str>,
    ) -> Result<ResourceEntry, CoreError> {
        Err(CoreError::PersonalReadOnly(Self::config_path()))
    }

    fn place(&self) -> Result<&ProjectFolder, CoreError> {
        Err(CoreError::PersonalHasNoFiles)
    }

    fn canonical_name(&self) -> String {
        ProjectSlug::PERSONAL.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::Personal;

    #[test]
    fn a_context_name_becomes_a_resource_name() {
        assert_eq!(Personal::resource_name("kind-synt"), "kind-synt");
        assert_eq!(
            Personal::resource_name("arn:aws:eks:eu-west-1:123:cluster/orders"),
            "arn-aws-eks-eu-west-1-123-cluster-orders"
        );
        assert_eq!(Personal::resource_name("..hidden"), "hidden");
        assert_eq!(Personal::resource_name(&"x".repeat(80)).len(), 64);
    }
}
