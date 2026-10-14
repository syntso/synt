use kube::Client;
use kube::discovery::{ApiGroup, Discovery};

use crate::{Kind, KindName, KubeError};

const FIRST_GROUPS: [&str; 4] = ["", "apps", "batch", "networking.k8s.io"];

#[derive(Debug, Clone, Default)]
pub struct KindCatalog {
    kinds: Vec<Kind>,
}

impl KindCatalog {
    pub(crate) async fn discover(client: &Client) -> Result<Self, KubeError> {
        let discovery = match Discovery::new(client.clone()).run_aggregated().await {
            Ok(discovery) => discovery,
            Err(_) => Discovery::new(client.clone()).run().await?,
        };

        let found = discovery
            .groups()
            .flat_map(ApiGroup::recommended_resources)
            .map(|(resource, capabilities)| Kind::discovered(resource, &capabilities));

        Ok(Self::from_kinds(found))
    }

    pub(crate) fn from_kinds(kinds: impl IntoIterator<Item = Kind>) -> Self {
        let mut kinds: Vec<Kind> = kinds.into_iter().collect();
        kinds.sort_by_key(|kind| (group_rank(kind.group()), kind.group().to_owned()));

        Self { kinds }
    }

    pub fn find(&self, wanted: &KindName) -> Result<Kind, KubeError> {
        self.kinds
            .iter()
            .filter(|kind| wanted.group().is_none_or(|group| kind.group() == group))
            .find(|kind| kind.is_named(wanted))
            .cloned()
            .ok_or_else(|| KubeError::UnknownKind(wanted.to_string()))
    }

    pub fn by_api_version(&self, api_version: &str, kind_name: &str) -> Result<Kind, KubeError> {
        let (group, version) = match api_version.split_once('/') {
            Some((group, version)) => (group, version),
            None => ("", api_version),
        };

        self.kinds
            .iter()
            .find(|kind| kind.group() == group && kind.name() == kind_name)
            .map(|kind| kind.with_version(version))
            .ok_or_else(|| KubeError::UnknownKind(format!("{kind_name} in {api_version}")))
    }
}

fn group_rank(group: &str) -> usize {
    FIRST_GROUPS
        .iter()
        .position(|first| *first == group)
        .unwrap_or(FIRST_GROUPS.len())
}

#[cfg(test)]
mod tests {
    use kube::discovery::ApiResource;

    use super::KindCatalog;
    use crate::{Kind, KindName};

    fn resource(group: &str, kind: &str, plural: &str) -> ApiResource {
        let api_version = if group.is_empty() {
            "v1".to_owned()
        } else {
            format!("{group}/v1")
        };

        ApiResource {
            group: group.to_owned(),
            version: "v1".to_owned(),
            api_version,
            kind: kind.to_owned(),
            plural: plural.to_owned(),
        }
    }

    fn name(text: &str) -> KindName {
        KindName::try_from(text).expect("a kind name")
    }

    fn kinds() -> KindCatalog {
        KindCatalog::from_kinds([
            Kind::new(resource("example.com", "Event", "events"), true),
            Kind::new(resource("events.k8s.io", "Event", "events"), true),
            Kind::new(resource("", "Event", "events"), true),
            Kind::new(resource("", "Pod", "pods"), true),
            Kind::new(resource("apps", "Deployment", "deployments"), true),
            Kind::new(resource("", "Namespace", "namespaces"), false),
        ])
    }

    #[test]
    fn a_kind_is_found_by_name_plural_or_short_name_in_any_case() {
        let kinds = kinds();

        for text in [
            "Deployment",
            "deployment",
            "deployments",
            "deploy",
            "DEPLOY",
        ] {
            assert_eq!(
                kinds.find(&name(text)).expect(text).to_string(),
                "Deployment.apps"
            );
        }
    }

    #[test]
    fn the_core_group_wins_when_several_groups_share_a_name() {
        let found = kinds().find(&name("events")).expect("events");

        assert_eq!(found.group(), "");
    }

    #[test]
    fn a_group_after_the_dot_picks_that_group() {
        let found = kinds().find(&name("events.events.k8s.io")).expect("events");

        assert_eq!(found.group(), "events.k8s.io");
    }

    #[test]
    fn an_unknown_kind_is_an_error_that_names_it() {
        let error = kinds().find(&name("gadgets")).expect_err("no gadgets");

        assert!(error.to_string().contains("gadgets"));
    }

    #[test]
    fn an_api_version_pins_the_group_and_version() {
        let found = kinds()
            .by_api_version("apps/v1beta2", "Deployment")
            .expect("deployment");

        assert_eq!(found.api_version(), "apps/v1beta2");
        assert!(found.is_namespaced());
    }

    #[test]
    fn cluster_scoped_kinds_say_so() {
        assert!(
            !kinds()
                .find(&name("ns"))
                .expect("namespaces")
                .is_namespaced()
        );
    }
}
