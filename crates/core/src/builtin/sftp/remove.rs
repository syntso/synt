use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::files::per_call_risk;
use super::{Files, PathParams};

#[derive(Debug, Clone)]
pub struct SftpRemove {
    files: Files,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Removed {
    pub path: String,
    pub kind: &'static str,
}

impl SftpRemove {
    pub(super) fn new(files: Files) -> Self {
        Self { files }
    }
}

impl Action for SftpRemove {
    const ID: &'static str = "sftp.remove";
    const RISK: Risk = Risk::Write;

    type Params = PathParams;
    type Output = Removed;

    fn risk(&self, target: Option<&ResourceRef>, _params: &PathParams) -> Risk {
        per_call_risk(target, Self::RISK)
    }

    fn summary(&self, output: &Removed) -> Value {
        json!({ "path": output.path, "kind": output.kind })
    }

    async fn run(&self, context: &Context, params: PathParams) -> Result<Removed, ActionError> {
        let path = params.path.as_str();

        let kind = self
            .files
            .run(context, move |sftp| async move { sftp.remove(path).await })
            .await?;

        Ok(Removed {
            path: params.path.into_string(),
            kind: kind.name(),
        })
    }
}
