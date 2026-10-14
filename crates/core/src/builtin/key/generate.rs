use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_ssh::StoredKey;

use super::KeyKind;
use crate::blocking::blocking;
use crate::ssh_keys::{KeyInfo, KeyName, SshKeys};

const COMMENT_PREFIX: &str = "synt:";

#[derive(Clone)]
pub struct KeyGenerate {
    keys: SshKeys,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateParams {
    pub name: KeyName,

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

    fn summary(&self, output: &KeyInfo) -> Value {
        output.summary()
    }

    async fn run(
        &self,
        _context: &Context,
        params: GenerateParams,
    ) -> Result<KeyInfo, ActionError> {
        let keys = self.keys.clone();

        blocking(move || {
            let comment = format!("{COMMENT_PREFIX}{}", params.name);
            let key = match params.kind {
                KeyKind::Ed25519 => StoredKey::generate_ed25519(&comment)?,
            };

            keys.store(&params.name, &key)
        })
        .await
    }
}
