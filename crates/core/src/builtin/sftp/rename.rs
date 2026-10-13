use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::files::per_call_risk;
use super::{Files, RemotePath};

#[derive(Debug, Clone)]
pub struct SftpRename {
    files: Files,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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
    const RISK: Risk = Risk::Write;

    type Params = RenameParams;
    type Output = Renamed;

    fn risk(&self, target: Option<&ResourceRef>, _params: &RenameParams) -> Risk {
        per_call_risk(target, Self::RISK)
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
