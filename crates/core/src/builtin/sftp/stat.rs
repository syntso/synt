use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::files::per_call_risk;
use super::{FileInfo, Files, PathParams};

#[derive(Debug, Clone)]
pub struct SftpStat {
    files: Files,
}

impl SftpStat {
    pub(super) fn new(files: Files) -> Self {
        Self { files }
    }
}

impl Action for SftpStat {
    const ID: &'static str = "sftp.stat";
    const RISK: Risk = Risk::Read;

    type Params = PathParams;
    type Output = FileInfo;

    fn risk(&self, target: Option<&ResourceRef>, _params: &PathParams) -> Risk {
        per_call_risk(target, Self::RISK)
    }

    fn summary(&self, output: &FileInfo) -> Value {
        json!({ "path": output.name, "kind": output.kind, "size": output.size })
    }

    async fn run(&self, context: &Context, params: PathParams) -> Result<FileInfo, ActionError> {
        let path = params.path.as_str();

        let entry = self
            .files
            .run(context, move |sftp| async move { sftp.stat(path).await })
            .await?;

        Ok(FileInfo::from(entry))
    }
}
