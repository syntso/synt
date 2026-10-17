mod action_name;
mod api;
mod asset_path;
mod capabilities;
mod contributions;
mod description;
mod event_topic;
mod extension_id;
mod http_host;
mod local_id;
mod secret_name;
mod unchecked;

use serde::{Deserialize, Serialize};

pub use action_name::ActionName;
pub use api::Api;
pub use asset_path::AssetPath;
pub use capabilities::Capabilities;
pub use contributions::{Contributions, Panel, SettingsSchema, Suffix};
pub use description::Description;
pub use event_topic::EventTopic;
pub use extension_id::ExtensionId;
pub use http_host::HttpHost;
pub use local_id::LocalId;
pub use secret_name::SecretName;

use synt_proto::OneLine;
use unchecked::Unchecked;

use crate::{ExtensionError, ManifestError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Unchecked")]
pub struct Manifest {
    id: ExtensionId,
    name: OneLine,
    version: OneLine,
    api: Api,
    description: Description,

    #[serde(skip_serializing_if = "Option::is_none")]
    repository: Option<String>,

    capabilities: Capabilities,

    #[serde(skip_serializing_if = "Contributions::is_empty")]
    contributes: Contributions,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, ExtensionError> {
        Ok(toml::from_str(text)?)
    }

    pub fn to_toml(&self) -> Result<String, ExtensionError> {
        Ok(toml::to_string(self)?)
    }

    pub(crate) fn id(&self) -> &ExtensionId {
        &self.id
    }

    pub(crate) fn name(&self) -> &OneLine {
        &self.name
    }

    pub(crate) fn version(&self) -> &OneLine {
        &self.version
    }

    pub(crate) fn api(&self) -> Api {
        self.api
    }

    pub(crate) fn description(&self) -> &Description {
        &self.description
    }

    pub(crate) fn repository(&self) -> Option<&str> {
        self.repository.as_deref()
    }

    pub(crate) fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    pub(crate) fn contributes(&self) -> &Contributions {
        &self.contributes
    }

    pub(crate) fn document_kind(&self, suffix: &str) -> Option<String> {
        self.contributes
            .document_type(suffix)
            .map(|kind| kind.kind_name(self.id.as_str()))
    }

    fn check(&self) -> Result<(), ManifestError> {
        self.check_api()?;
        self.check_calls()?;
        self.capabilities.check_secret_hosts()?;
        self.check_documents()?;
        self.contributes.check()?;

        self.check_contributed_actions()
    }

    fn check_api(&self) -> Result<(), ManifestError> {
        let needs_newer = self.capabilities.uses_api_0_2() || !self.contributes.is_empty();

        if self.api == Api::V0_1 && needs_newer {
            return Err(ManifestError::NeedsNewerApi);
        }

        Ok(())
    }

    fn check_calls(&self) -> Result<(), ManifestError> {
        let own = self
            .capabilities
            .calls()
            .find(|action| action.namespace() == self.id.as_str());

        match own {
            Some(action) => Err(ManifestError::CallsItself(action.to_string())),
            None => Ok(()),
        }
    }

    fn check_documents(&self) -> Result<(), ManifestError> {
        let granted = self.capabilities.documents();
        let contributed = self.contributes.has_documents();

        match (granted, contributed) {
            (true, false) => Err(ManifestError::DocumentsWithoutType),
            (false, true) => Err(ManifestError::TypeWithoutDocuments),
            _ => Ok(()),
        }
    }

    fn check_contributed_actions(&self) -> Result<(), ManifestError> {
        let prefix = format!("{}.", self.id);

        match self
            .contributes
            .actions()
            .find(|action| !action.starts_with(&prefix))
        {
            Some(action) => Err(ManifestError::ForeignAction(action.to_owned())),
            None => Ok(()),
        }
    }
}

impl TryFrom<Unchecked> for Manifest {
    type Error = ManifestError;

    fn try_from(fields: Unchecked) -> Result<Self, ManifestError> {
        let Unchecked {
            id,
            name,
            version,
            api,
            description,
            repository,
            capabilities,
            contributes,
        } = fields;

        let manifest = Self {
            id,
            name,
            version,
            api,
            description,
            repository,
            capabilities,
            contributes,
        };

        manifest.check()?;
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::Manifest;
    use crate::files::FileAccess;
    use crate::manifest::{Api, EventTopic};

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

        assert_eq!(manifest.id().as_str(), "hello");
        assert_eq!(manifest.api(), Api::V0_1);
        assert!(manifest.capabilities().storage());
        assert!(manifest.capabilities().may_call("ssh.exec"));
        assert!(!manifest.capabilities().may_call("sftp.write"));
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

        assert!(manifest.capabilities().calls().next().is_none());
        assert!(!manifest.capabilities().storage());
        assert!(manifest.contributes().is_empty());
    }

    #[test]
    fn reads_every_capability_and_contribution_of_api_0_2() {
        let manifest = Manifest::parse(BOARD).unwrap();
        let capabilities = manifest.capabilities();

        assert_eq!(manifest.api(), Api::V0_2);
        assert!(capabilities.reaches("127.0.0.1:3000"));
        assert!(capabilities.may_attach("grafana/token", "grafana.example.com"));
        assert!(!capabilities.may_attach("grafana/token", "127.0.0.1:3000"));
        assert!(capabilities.hears(EventTopic::Approval));
        assert!(!capabilities.hears(EventTopic::Session));
        assert_eq!(capabilities.files(), Some(FileAccess::ReadWrite));

        assert_eq!(
            manifest
                .contributes()
                .views()
                .into_iter()
                .collect::<Vec<_>>(),
            ["alerts", "boards"]
        );
        assert_eq!(
            manifest.document_kind("uml").as_deref(),
            Some("ext.board.diagram")
        );
        assert!(manifest.contributes().web_panel("editor").is_some());
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
