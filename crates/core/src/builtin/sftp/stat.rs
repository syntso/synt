use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::{FileInfo, Files, PathParams};

#[derive(Debug, Clone)]
pub(crate) struct SftpStat {
    files: Files,
}

impl SftpStat {
    pub(super) fn new(files: Files) -> Self {
        Self { files }
    }
}

impl Action for SftpStat {
    const ID: &'static str = "sftp.stat";
    const TITLE: &'static str = "Show what a path on a host is";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = PathParams;
    type Output = FileInfo;

    fn risk(&self, target: Option<&ResourceRef>, _params: &PathParams) -> Risk {
        self.files.per_call_risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &PathParams) -> Option<String> {
        self.files.destination(target)
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
