use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::{Files, PathParams};

#[derive(Debug, Clone)]
pub(crate) struct SftpMkdir {
    files: Files,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Made {
    pub path: String,
}

impl SftpMkdir {
    pub(super) fn new(files: Files) -> Self {
        Self { files }
    }
}

impl Action for SftpMkdir {
    const ID: &'static str = "sftp.mkdir";
    const TITLE: &'static str = "Make a directory on a host";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = PathParams;
    type Output = Made;

    fn risk(&self, target: Option<&ResourceRef>, _params: &PathParams) -> Risk {
        self.files.per_call_risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &PathParams) -> Option<String> {
        self.files.destination(target)
    }

    fn summary(&self, output: &Made) -> Value {
        json!({ "path": output.path })
    }

    async fn run(&self, context: &Context, params: PathParams) -> Result<Made, ActionError> {
        let path = params.path.as_str();

        self.files
            .run(context, move |sftp| async move { sftp.mkdir(path).await })
            .await?;

        Ok(Made {
            path: params.path.into_string(),
        })
    }
}
