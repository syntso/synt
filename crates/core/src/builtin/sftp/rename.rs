use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::{Files, RemotePath};

#[derive(Debug, Clone)]
pub struct SftpRename {
    files: Files,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct RenameParams {
    pub from: RemotePath,
    pub to: RemotePath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Renamed {
    pub from: String,
    pub to: String,
}

impl SftpRename {
    pub(super) fn new(files: Files) -> Self {
        Self { files }
    }
}

impl Action for SftpRename {
    const ID: &'static str = "sftp.rename";
    const TITLE: &'static str = "Rename a path on a host";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = RenameParams;
    type Output = Renamed;

    fn risk(&self, target: Option<&ResourceRef>, _params: &RenameParams) -> Risk {
        self.files.per_call_risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &RenameParams) -> Option<String> {
        self.files.destination(target)
    }

    fn summary(&self, output: &Renamed) -> Value {
        json!({ "from": output.from, "to": output.to })
    }

    async fn run(&self, context: &Context, params: RenameParams) -> Result<Renamed, ActionError> {
        let (from, to) = (params.from.as_str(), params.to.as_str());

        self.files
            .run(
                context,
                move |sftp| async move { sftp.rename(from, to).await },
            )
            .await?;

        Ok(Renamed {
            from: params.from.into_string(),
            to: params.to.into_string(),
        })
    }
}
