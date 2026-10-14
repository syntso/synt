use std::fmt;

const LONGEST_NAME: usize = 64;
const SECRET_PREFIX: &str = "secret://";
const VAULT_PREFIX: &str = "ssh-key.";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyName(String);

impl KeyName {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.is_empty() || text.len() > LONGEST_NAME {
            return Err(format!("a key name must be 1 to {LONGEST_NAME} characters"));
        }

        if !text.split('/').all(is_segment) {
            return Err(format!(
                "{text:?} is not a valid key name: use letters, digits, '-' and '_', \
                 with '/' between parts"
            ));
        }

        Ok(Self(text.to_owned()))
    }

    pub fn from_reference(identity: &str) -> Option<Result<Self, String>> {
        let name = identity.strip_prefix(SECRET_PREFIX)?;

        Some(Self::parse(name).map_err(|reason| format!("identity {identity:?}: {reason}")))
    }

    pub fn reference(name: &str) -> String {
        format!("{SECRET_PREFIX}{name}")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn vault_item(&self) -> String {
        format!("{VAULT_PREFIX}{}", self.0.replace('/', "."))
    }
}

impl fmt::Display for KeyName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn is_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_".contains(character))
}

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
            KeyName::from_reference("secret://night").map(|name| name.map(|name| name.to_string())),
            Some(Ok("night".to_owned()))
        );
        assert!(matches!(KeyName::from_reference("secret://"), Some(Err(_))));
    }
}
