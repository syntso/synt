use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::files::per_call_risk;
use super::{Files, PathParams};

#[derive(Debug, Clone)]
pub struct SftpMkdir {
    files: Files,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Made {
    pub path: String,
}

impl SftpMkdir {
    pub(super) fn new(files: Files) -> Self {
        Self { files }
    }
}

impl Action for SftpMkdir {
    const ID: &'static str = "sftp.mkdir";
    const RISK: Risk = Risk::Write;

    type Params = PathParams;
    type Output = Made;

    fn risk(&self, target: Option<&ResourceRef>, _params: &PathParams) -> Risk {
        per_call_risk(target, Self::RISK)
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
