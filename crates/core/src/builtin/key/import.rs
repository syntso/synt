use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk, blocking};
use synt_journal::ResourceRef;
use synt_ssh::StoredKey;

use crate::CoreError;
use crate::ssh_keys::{KeyInfo, KeyName, SshKeys};

#[derive(Clone)]
pub struct KeyImport {
    keys: SshKeys,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportParams {
    pub name: KeyName,
    pub private_key_path: PathBuf,
}

impl KeyImport {
    pub fn new(keys: SshKeys) -> Self {
        Self { keys }
    }
}

impl Action for KeyImport {
    const ID: &'static str = "key.import";
    const TITLE: &'static str = "Import an SSH private key file into the core's vault";
    const RISK: Risk = Risk::Privileged;
    const PERSON_ONLY: bool = true;

    type Params = ImportParams;
    type Output = KeyInfo;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &ImportParams,
    ) -> Result<(), ActionError> {
        if !params.private_key_path.is_absolute() {
            return Err(ActionError::InvalidParams(
                "private_key_path must be an absolute path".to_owned(),
            ));
        }

        Ok(())
    }

    fn summary(&self, output: &KeyInfo) -> Value {
        output.summary()
    }

    async fn run(&self, _context: &Context, params: ImportParams) -> Result<KeyInfo, ActionError> {
        let key = StoredKey::import(&params.private_key_path)
            .await
            .map_err(CoreError::KeyImport)?;

        let keys = self.keys.clone();

        blocking(move || keys.store(&params.name, &key)).await
    }
}
