use crate::vault::{VaultKind, VaultName};

pub enum SecretItem {}

impl VaultKind for SecretItem {
    const NOUN: &'static str = "secret";
    const VAULT_PREFIX: &'static str = "secret.";
}

pub type SecretName = VaultName<SecretItem>;

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
