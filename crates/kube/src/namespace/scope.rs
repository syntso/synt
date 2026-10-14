use schemars::JsonSchema;
use serde::Deserialize;

use super::Namespace;
use crate::KubeError;

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, JsonSchema)]
#[serde(try_from = "ScopeFields")]
pub enum NamespaceScope {
    #[default]
    Default,
    Only(Namespace),
    All,
}

impl NamespaceScope {
    pub fn resolve(&self, default: &str) -> Option<String> {
        match self {
            Self::Default => Some(default.to_owned()),
            Self::Only(namespace) => Some(namespace.to_string()),
            Self::All => None,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
struct ScopeFields {
    #[serde(default)]
    namespace: Option<Namespace>,

    #[serde(default)]
    all_namespaces: bool,
}

impl TryFrom<ScopeFields> for NamespaceScope {
    type Error = KubeError;

    fn try_from(fields: ScopeFields) -> Result<Self, KubeError> {
        match (fields.namespace, fields.all_namespaces) {
            (Some(_), true) => Err(KubeError::invalid(
                "give a namespace or all_namespaces, not both",
            )),
            (None, true) => Ok(Self::All),
            (Some(namespace), false) => Ok(Self::Only(namespace)),
            (None, false) => Ok(Self::Default),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::NamespaceScope;

    fn scope(fields: serde_json::Value) -> Result<NamespaceScope, serde_json::Error> {
        serde_json::from_value(fields)
    }

    #[test]
    fn the_fields_pick_one_scope() {
        assert_eq!(scope(json!({})).unwrap(), NamespaceScope::Default);
        assert_eq!(
            scope(json!({ "all_namespaces": true })).unwrap(),
            NamespaceScope::All
        );
        assert_eq!(
            scope(json!({ "namespace": "orders" }))
                .unwrap()
                .resolve("default"),
            Some("orders".to_owned())
        );
    }

    #[test]
    fn a_namespace_and_all_namespaces_do_not_go_together() {
        assert!(scope(json!({ "namespace": "orders", "all_namespaces": true })).is_err());
        assert!(scope(json!({ "namespace": "Orders" })).is_err());
    }

    #[test]
    fn the_default_scope_is_the_namespace_of_the_context() {
        assert_eq!(
            NamespaceScope::Default.resolve("shop"),
            Some("shop".to_owned())
        );
        assert_eq!(NamespaceScope::All.resolve("shop"), None);
    }
}
