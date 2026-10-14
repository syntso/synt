use serde::{Deserialize, Serialize};
use synt_journal::ActionId;

use super::Capabilities;
use crate::ExtensionError;

const SUPPORTED_API: &str = "0.1";

const LONGEST_ID: usize = 40;
const LONGEST_TEXT: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api: String,

    #[serde(default)]
    pub description: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    #[serde(default)]
    pub capabilities: Capabilities,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, ExtensionError> {
        let manifest: Self =
            toml::from_str(text).map_err(|error| ExtensionError::Manifest(error.to_string()))?;

        manifest.check()?;
        Ok(manifest)
    }

    pub fn to_toml(&self) -> Result<String, ExtensionError> {
        toml::to_string(self).map_err(|error| ExtensionError::Manifest(error.to_string()))
    }

    pub fn check(&self) -> Result<(), ExtensionError> {
        check_id(&self.id)?;
        check_text("name", &self.name)?;
        check_text("version", &self.version)?;
        check_description(&self.description)?;

        if self.api != SUPPORTED_API {
            return Err(refused(format!(
                "api {} is not supported; this core runs extensions for api {SUPPORTED_API}",
                self.api
            )));
        }

        self.capabilities
            .call
            .iter()
            .try_for_each(|action| self.check_call(action))
    }

    fn check_call(&self, action: &str) -> Result<(), ExtensionError> {
        ActionId::new(action).map_err(|error| refused(format!("call {action:?}: {error}")))?;

        if action.contains(['*', '?']) {
            return Err(refused(format!(
                "call {action:?}: name one action, not a pattern"
            )));
        }

        if action.split('.').next() == Some(self.id.as_str()) {
            return Err(refused(format!(
                "call {action:?}: an extension runs its own actions without calling them"
            )));
        }

        Ok(())
    }
}

fn check_id(id: &str) -> Result<(), ExtensionError> {
    let starts_with_letter = id.starts_with(|first: char| first.is_ascii_lowercase());
    let plain = id.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    });

    if !starts_with_letter || !plain || id.len() > LONGEST_ID {
        return Err(refused(format!(
            "id {id:?} must start with a lowercase letter and hold only lowercase letters, \
             digits and dashes, at most {LONGEST_ID} of them"
        )));
    }

    Ok(())
}

fn check_text(field: &str, text: &str) -> Result<(), ExtensionError> {
    if text.trim().is_empty() || text.len() > LONGEST_TEXT || text.contains(char::is_control) {
        return Err(refused(format!(
            "{field} must be one line of at most {LONGEST_TEXT} bytes"
        )));
    }

    Ok(())
}

fn check_description(description: &str) -> Result<(), ExtensionError> {
    if description.len() > LONGEST_TEXT * 5 || description.contains(char::is_control) {
        return Err(refused(
            "description is too long or holds control characters",
        ));
    }

    Ok(())
}

fn refused(reason: impl Into<String>) -> ExtensionError {
    ExtensionError::Manifest(reason.into())
}

#[cfg(test)]
mod tests {
    use super::Manifest;

    const HELLO: &str = r#"
id = "hello"
name = "Hello"
version = "0.1.0"
api = "0.1"
description = "Greets a host."

[capabilities]
call = ["ssh.exec"]
storage = true
"#;

    #[test]
    fn reads_a_manifest() {
        let manifest = Manifest::parse(HELLO).unwrap();

        assert_eq!(manifest.id, "hello");
        assert!(manifest.capabilities.storage);
        assert!(manifest.capabilities.may_call("ssh.exec"));
        assert!(!manifest.capabilities.may_call("sftp.write"));
    }

    #[test]
    fn round_trips_through_toml() {
        let manifest = Manifest::parse(HELLO).unwrap();
        let again = Manifest::parse(&manifest.to_toml().unwrap()).unwrap();

        assert_eq!(again, manifest);
    }

    #[test]
    fn capabilities_default_to_nothing() {
        let manifest =
            Manifest::parse("id = \"quiet\"\nname = \"Quiet\"\nversion = \"1\"\napi = \"0.1\"")
                .unwrap();

        assert!(manifest.capabilities.call.is_empty());
        assert!(!manifest.capabilities.storage);
    }

    #[test]
    fn refuses_other_api_versions() {
        let text = HELLO.replace("api = \"0.1\"", "api = \"9.0\"");

        assert!(Manifest::parse(&text).is_err());
    }

    #[test]
    fn refuses_odd_ids() {
        for id in ["Hello", "1st", "with space", "dot.ted", ""] {
            let text = HELLO.replace("id = \"hello\"", &format!("id = {id:?}"));

            assert!(Manifest::parse(&text).is_err(), "{id}");
        }
    }

    #[test]
    fn refuses_patterns_and_own_actions_in_call() {
        for call in ["ssh.*", "hello.greet"] {
            let text = HELLO.replace("\"ssh.exec\"", &format!("{call:?}"));

            assert!(Manifest::parse(&text).is_err(), "{call}");
        }
    }
}
