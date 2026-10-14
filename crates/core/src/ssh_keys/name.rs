use crate::vault::{VaultKind, VaultName};

pub enum SshKeyItem {}

impl VaultKind for SshKeyItem {
    const NOUN: &'static str = "key";
    const VAULT_PREFIX: &'static str = "ssh-key.";
}

pub type KeyName = VaultName<SshKeyItem>;

#[cfg(test)]
mod tests {
    use super::KeyName;

    #[test]
    fn names_with_parts_map_to_distinct_vault_items() {
        let deploy = KeyName::parse("prod/deploy").expect("valid");

        assert_eq!(deploy.vault_item(), "ssh-key.prod.deploy");
    }

    #[test]
    fn names_that_could_escape_the_vault_are_refused() {
        for name in ["", "../x", "a//b", "/a", "a/", "a.b", "a b", "a\\b"] {
            assert!(KeyName::parse(name).is_err(), "{name:?}");
        }
    }

    #[test]
    fn references_need_the_secret_prefix() {
        assert!(KeyName::from_reference("~/.ssh/id_ed25519").is_none());
        assert_eq!(
            KeyName::from_reference("secret://night")
                .map(|name| name.map(|name| name.to_string()).ok()),
            Some(Some("night".to_owned()))
        );
        assert!(matches!(KeyName::from_reference("secret://"), Some(Err(_))));
    }
}
