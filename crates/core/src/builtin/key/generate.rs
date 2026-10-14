use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::StoredKey;

use super::KeyKind;
use crate::builtin::blocking::blocking;
use crate::ssh_keys::{KeyInfo, KeyName, SshKeys};

const COMMENT_PREFIX: &str = "synt:";

#[derive(Clone)]
pub struct KeyGenerate {
    keys: SshKeys,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateParams {
    pub name: String,

    #[serde(default)]
    pub kind: KeyKind,
}

impl KeyGenerate {
    pub fn new(keys: SshKeys) -> Self {
        Self { keys }
    }
}

impl Action for KeyGenerate {
    const ID: &'static str = "key.generate";
    const TITLE: &'static str =
        "Make a new SSH key inside the core's vault and show its public key";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = GenerateParams;
    type Output = KeyInfo;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &GenerateParams,
    ) -> Result<(), ActionError> {
        KeyName::parse(&params.name)
            .map(|_| ())
            .map_err(ActionError::InvalidParams)
    }

    fn summary(&self, output: &KeyInfo) -> Value {
        output.summary()
    }

    async fn run(
        &self,
        _context: &Context,
        params: GenerateParams,
    ) -> Result<KeyInfo, ActionError> {
        let name = KeyName::parse(&params.name).map_err(ActionError::InvalidParams)?;
        let keys = self.keys.clone();

        blocking(move || {
            let key = match params.kind {
                KeyKind::Ed25519 => {
                    StoredKey::generate_ed25519(&format!("{COMMENT_PREFIX}{name}"))?
                }
            };

            keys.store(&name, &key)
        })
        .await
    }
}
