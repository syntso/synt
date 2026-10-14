use std::fmt;

use crate::ssh_keys::KeyName;

const SECRET_PREFIX: &str = "secret://";
const VAULT_PREFIX: &str = "password.";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PasswordName(String);

impl PasswordName {
    pub fn parse(text: &str) -> Result<Self, String> {
        KeyName::parse(text)
            .map(|name| Self(name.as_str().to_owned()))
            .map_err(|reason| reason.replace("key name", "password name"))
    }

    pub fn from_reference(reference: &str) -> Option<Result<Self, String>> {
        let name = reference.strip_prefix(SECRET_PREFIX)?;

        Some(Self::parse(name))
    }

    pub fn reference(&self) -> String {
        format!("{SECRET_PREFIX}{}", self.0)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn vault_item(&self) -> String {
        format!("{VAULT_PREFIX}{}", self.0.replace('/', "."))
    }
}

impl fmt::Display for PasswordName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::PasswordName;

    #[test]
    fn a_reference_names_a_vault_item_of_its_own_kind() {
        let name = PasswordName::from_reference("secret://prod/orders-db")
            .expect("a reference")
            .expect("a valid name");

        assert_eq!(name.as_str(), "prod/orders-db");
        assert_eq!(name.vault_item(), "password.prod.orders-db");
        assert_eq!(name.reference(), "secret://prod/orders-db");
    }

    #[test]
    fn plain_text_is_not_a_reference() {
        assert!(PasswordName::from_reference("hunter2").is_none());
    }

    #[test]
    fn names_follow_the_key_name_rules() {
        assert!(PasswordName::parse("../escape").is_err());
        assert!(PasswordName::parse("").is_err());
    }
}
