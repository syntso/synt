use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::{FileInfo, Files, PathParams};

#[derive(Debug, Clone)]
pub(crate) struct SftpList {
    files: Files,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Listing {
    pub path: String,
    pub entries: Vec<FileInfo>,
    pub truncated: bool,
}

impl SftpList {
    pub(super) fn new(files: Files) -> Self {
        Self { files }
    }
}

impl Action for SftpList {
    const ID: &'static str = "sftp.list";
    const TITLE: &'static str = "List a directory on a host";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = PathParams;
    type Output = Listing;

    fn risk(&self, target: Option<&ResourceRef>, _params: &PathParams) -> Risk {
        self.files.per_call_risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &PathParams) -> Option<String> {
        self.files.destination(target)
    }

    fn summary(&self, output: &Listing) -> Value {
        json!({
            "path": output.path,
            "entries": output.entries.len(),
            "truncated": output.truncated,
        })
    }

    async fn run(&self, context: &Context, params: PathParams) -> Result<Listing, ActionError> {
        let path = params.path.as_str();

        let directory = self
            .files
            .run(context, move |sftp| async move { sftp.list(path).await })
            .await?;

        Ok(Listing {
            path: params.path.into_string(),
            entries: directory.entries.into_iter().map(FileInfo::from).collect(),
            truncated: directory.truncated,
        })
    }
}
