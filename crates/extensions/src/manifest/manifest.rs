use serde::{Deserialize, Serialize};
use synt_journal::ActionId;

use super::checks::{LONGEST_TEXT, check_text, refused};
use super::{Api, Capabilities, Contributions};
use crate::ExtensionError;

const LONGEST_ID: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api: Api,

    #[serde(default)]
    pub description: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    #[serde(default)]
    pub capabilities: Capabilities,

    #[serde(default, skip_serializing_if = "Contributions::is_empty")]
    pub contributes: Contributions,
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

        self.check_api()?;
        self.check_calls()?;
        self.capabilities.check_http()?;
        self.check_documents()?;

        self.contributes.check()?;
        self.check_contributed_actions()
    }

    pub fn document_kind(&self, suffix: &str) -> Option<String> {
        self.contributes
            .document_type(suffix)
            .map(|kind| kind.kind_name(&self.id))
    }

    fn check_api(&self) -> Result<(), ExtensionError> {
        let needs_newer = self.capabilities.uses_api_0_2() || !self.contributes.is_empty();

        if self.api == Api::V0_1 && needs_newer {
            return Err(refused(
                "contributes and the capabilities http, secrets, documents, events, timers \
                 and files need api 0.2",
            ));
        }

        Ok(())
    }

    fn check_calls(&self) -> Result<(), ExtensionError> {
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

    fn check_documents(&self) -> Result<(), ExtensionError> {
        let contributed = !self.contributes.document.is_empty();

        match (self.capabilities.documents, contributed) {
            (true, false) => Err(refused(
                "capabilities.documents needs at least one contributes.document type",
            )),
            (false, true) => Err(refused(
                "contributes.document needs documents = true in capabilities",
            )),
            _ => Ok(()),
        }
    }

    fn check_contributed_actions(&self) -> Result<(), ExtensionError> {
        let prefix = format!("{}.", self.id);

        match self
            .contributes
            .actions()
            .find(|action| !action.starts_with(&prefix))
        {
            Some(action) => Err(refused(format!(
                "contributes names the action {action}; commands and menus run the extension's \
                 own actions only"
            ))),
            None => Ok(()),
        }
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

fn check_description(description: &str) -> Result<(), ExtensionError> {
    if description.len() > LONGEST_TEXT * 5 || description.contains(char::is_control) {
        return Err(refused(
            "description is too long or holds control characters",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Manifest;
    use crate::manifest::{Api, EventTopic, FileAccess};

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

    const BOARD: &str = r#"
id = "board"
name = "Board"
version = "0.2.0"
api = "0.2"

[capabilities]
http = ["grafana.example.com", "127.0.0.1:3000"]
secrets = { "grafana/token" = ["grafana.example.com"] }
documents = true
events = ["journal", "approval"]
timers = true
files = "read-write"

[[contributes.sidebar]]
id = "boards"
title = "Boards"
view = "boards"

[[contributes.document]]
type = "diagram"
title = "Diagram"
suffix = "uml"
panel = "editor"

[[contributes.panel]]
id = "editor"
title = "Diagram editor"
entry = "panel/index.html"

[[contributes.command]]
title = "Refresh boards"
action = "board.refresh"

[[contributes.resource.menu]]
title = "Show load"
action = "board.load"
kinds = ["host"]

[[contributes.status]]
id = "alerts"
title = "Alerts"
view = "alerts"

[contributes.settings.schema]
type = "object"
properties = { url = { type = "string" } }
"#;

    #[test]
    fn reads_a_manifest() {
        let manifest = Manifest::parse(HELLO).unwrap();

        assert_eq!(manifest.id, "hello");
        assert_eq!(manifest.api, Api::V0_1);
        assert!(manifest.capabilities.storage);
        assert!(manifest.capabilities.may_call("ssh.exec"));
        assert!(!manifest.capabilities.may_call("sftp.write"));
    }

    #[test]
    fn round_trips_through_toml() {
        for text in [HELLO, BOARD] {
            let manifest = Manifest::parse(text).unwrap();
            let again = Manifest::parse(&manifest.to_toml().unwrap()).unwrap();

            assert_eq!(again, manifest);
        }
    }

    #[test]
    fn capabilities_default_to_nothing() {
        let manifest =
            Manifest::parse("id = \"quiet\"\nname = \"Quiet\"\nversion = \"1\"\napi = \"0.1\"")
                .unwrap();

        assert!(manifest.capabilities.call.is_empty());
        assert!(!manifest.capabilities.storage);
        assert!(manifest.contributes.is_empty());
    }

    #[test]
    fn reads_every_capability_and_contribution_of_api_0_2() {
        let manifest = Manifest::parse(BOARD).unwrap();
        let capabilities = &manifest.capabilities;

        assert_eq!(manifest.api, Api::V0_2);
        assert!(capabilities.reaches("127.0.0.1:3000"));
        assert!(capabilities.may_attach("grafana/token", "grafana.example.com"));
        assert!(!capabilities.may_attach("grafana/token", "127.0.0.1:3000"));
        assert!(capabilities.hears(EventTopic::Approval));
        assert!(!capabilities.hears(EventTopic::Session));
        assert_eq!(capabilities.files, Some(FileAccess::ReadWrite));

        assert_eq!(
            manifest.contributes.views().into_iter().collect::<Vec<_>>(),
            ["alerts", "boards"]
        );
        assert_eq!(
            manifest.document_kind("uml").as_deref(),
            Some("ext.board.diagram")
        );
        assert!(manifest.contributes.panel("editor").unwrap().is_web());
    }

    #[test]
    fn refuses_other_api_versions() {
        let text = HELLO.replace("api = \"0.1\"", "api = \"9.0\"");

        assert!(Manifest::parse(&text).is_err());
    }

    #[test]
    fn api_0_1_cannot_ask_for_the_new_capabilities() {
        let text = BOARD.replace("api = \"0.2\"", "api = \"0.1\"");

        let refusal = Manifest::parse(&text).unwrap_err().to_string();

        assert!(refusal.contains("need api 0.2"), "{refusal}");
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

    #[test]
    fn a_secret_goes_only_to_listed_hosts() {
        let text = BOARD.replace("[\"grafana.example.com\"] }", "[\"evil.example.com\"] }");

        let refusal = Manifest::parse(&text).unwrap_err().to_string();

        assert!(
            refusal.contains("not listed in capabilities.http"),
            "{refusal}"
        );
    }

    #[test]
    fn documents_need_a_type_and_types_need_the_capability() {
        let without_capability = BOARD.replace("documents = true\n", "");
        let without_types = HELLO.replace("api = \"0.1\"", "api = \"0.2\"") + "documents = true\n";

        assert!(Manifest::parse(&without_capability).is_err());
        assert!(Manifest::parse(&without_types).is_err());
    }

    #[test]
    fn commands_run_only_the_extensions_own_actions() {
        let text = BOARD.replace("action = \"board.refresh\"", "action = \"ssh.exec\"");

        let refusal = Manifest::parse(&text).unwrap_err().to_string();

        assert!(refusal.contains("own actions only"), "{refusal}");
    }

    #[test]
    fn a_panel_is_either_native_or_web() {
        let both = BOARD.replace(
            "entry = \"panel/index.html\"",
            "entry = \"panel/index.html\"\nview = \"editor\"",
        );
        let escaping = BOARD.replace("panel/index.html", "../index.html");

        assert!(Manifest::parse(&both).is_err());
        assert!(Manifest::parse(&escaping).is_err());
    }

    #[test]
    fn unknown_contribution_points_are_refused() {
        let text = BOARD.to_owned() + "\n[[contributes.toolbar]]\ntitle = \"x\"\n";

        assert!(Manifest::parse(&text).is_err());
    }
}
