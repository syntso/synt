use std::fmt;

use crate::ssh_keys::KeyName;

const SECRET_PREFIX: &str = "secret://";
const VAULT_PREFIX: &str = "secret.";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretName(String);

impl SecretName {
    pub fn parse(text: &str) -> Result<Self, String> {
        let checked =
            KeyName::parse(text).map_err(|reason| reason.replace("a key name", "a secret name"))?;

        Ok(Self(checked.as_str().to_owned()))
    }

    pub fn from_reference(reference: &str) -> Option<Result<Self, String>> {
        let name = reference.strip_prefix(SECRET_PREFIX)?;

        Some(Self::parse(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn reference(&self) -> String {
        KeyName::reference(&self.0)
    }

    pub(crate) fn vault_item(&self) -> String {
        format!("{VAULT_PREFIX}{}", self.0.replace('/', "."))
    }
}

impl fmt::Display for SecretName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::SecretName;

    #[test]
    fn secrets_and_ssh_keys_live_in_separate_vault_items() {
        let name = SecretName::parse("grafana/token").unwrap();

        assert_eq!(name.vault_item(), "secret.grafana.token");
        assert_eq!(name.reference(), "secret://grafana/token");
    }

    #[test]
    fn a_reference_names_the_secret_after_its_scheme() {
        let name = SecretName::from_reference("secret://prod/orders-db")
            .expect("a reference")
            .expect("a valid name");

        assert_eq!(name.as_str(), "prod/orders-db");
        assert!(SecretName::from_reference("hunter2").is_none());
        assert!(
            SecretName::from_reference("secret://../escape")
                .unwrap()
                .is_err()
        );
    }

    #[test]
    fn names_follow_the_key_rules() {
        for name in ["", "../x", "a.b", "a b"] {
            assert!(SecretName::parse(name).is_err(), "{name:?}");
        }
    }
}
