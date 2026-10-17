use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::Deserialize;

use crate::KubeError;

const SHORT_NAMES: [(&str, &str); 26] = [
    ("cm", "configmaps"),
    ("cj", "cronjobs"),
    ("crd", "customresourcedefinitions"),
    ("crds", "customresourcedefinitions"),
    ("csr", "certificatesigningrequests"),
    ("deploy", "deployments"),
    ("ds", "daemonsets"),
    ("ep", "endpoints"),
    ("ev", "events"),
    ("hpa", "horizontalpodautoscalers"),
    ("ing", "ingresses"),
    ("limits", "limitranges"),
    ("netpol", "networkpolicies"),
    ("no", "nodes"),
    ("ns", "namespaces"),
    ("pc", "priorityclasses"),
    ("pdb", "poddisruptionbudgets"),
    ("po", "pods"),
    ("pv", "persistentvolumes"),
    ("pvc", "persistentvolumeclaims"),
    ("quota", "resourcequotas"),
    ("rs", "replicasets"),
    ("sa", "serviceaccounts"),
    ("sc", "storageclasses"),
    ("sts", "statefulsets"),
    ("svc", "services"),
];

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(try_from = "String")]
pub struct KindName {
    text: String,
    name: String,
    group: Option<String>,
}

impl KindName {
    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn group(&self) -> Option<&str> {
        self.group.as_deref()
    }

    pub(crate) fn plural(&self) -> &str {
        SHORT_NAMES
            .iter()
            .find(|(short, _)| *short == self.name)
            .map_or(&self.name, |(_, plural)| plural)
    }

    fn is_valid(text: &str) -> bool {
        let allowed =
            |character: char| character.is_ascii_alphanumeric() || ".-".contains(character);

        !text.is_empty() && text.chars().all(allowed)
    }
}

impl TryFrom<String> for KindName {
    type Error = KubeError;

    fn try_from(text: String) -> Result<Self, KubeError> {
        if !Self::is_valid(&text) {
            return Err(KubeError::invalid(format!(
                "kind must be a kind, its plural or a short name, such as pods or deploy, \
                 got {text:?}"
            )));
        }

        let lowered = text.to_lowercase();
        let (name, group) = match lowered.split_once('.') {
            Some((name, group)) => (name.to_owned(), Some(group.to_owned())),
            None => (lowered, None),
        };

        Ok(Self { text, name, group })
    }
}

impl TryFrom<&str> for KindName {
    type Error = KubeError;

    fn try_from(text: &str) -> Result<Self, KubeError> {
        Self::try_from(text.to_owned())
    }
}

impl Display for KindName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::KindName;

    #[test]
    fn a_short_name_leads_to_its_plural() {
        let name = KindName::try_from("DEPLOY").unwrap();

        assert_eq!((name.name(), name.plural()), ("deploy", "deployments"));
        assert_eq!(name.as_str(), "DEPLOY");
    }

    #[test]
    fn a_group_follows_the_first_dot() {
        let name = KindName::try_from("events.events.k8s.io").unwrap();

        assert_eq!(
            (name.name(), name.group()),
            ("events", Some("events.k8s.io"))
        );
    }

    #[test]
    fn a_kind_with_a_path_or_spaces_is_refused() {
        for text in ["", "pods/exec", "pods ", "a?b"] {
            assert!(KindName::try_from(text).is_err(), "{text:?}");
        }
    }
}
