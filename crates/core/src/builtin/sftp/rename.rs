use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::{RemotePath, SftpActions};

#[derive(Debug, Clone)]
pub(crate) struct SftpRename {
    sftp: SftpActions,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct RenameParams {
    pub from: RemotePath,
    pub to: RemotePath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Renamed {
    pub from: String,
    pub to: String,
}

impl SftpRename {
    pub(super) fn new(sftp: SftpActions) -> Self {
        Self { sftp }
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
        self.sftp.per_call_risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &RenameParams) -> Option<String> {
        self.sftp.destination(target)
    }

    fn summary(&self, output: &Renamed) -> Value {
        json!({ "from": output.from, "to": output.to })
    }

    async fn run(&self, context: &Context, params: RenameParams) -> Result<Renamed, ActionError> {
        let (from, to) = (params.from.as_str(), params.to.as_str());

        self.sftp
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
