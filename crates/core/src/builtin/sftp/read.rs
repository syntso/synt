use data_encoding::BASE64;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::LARGEST_READ;
use super::{RemotePath, SftpActions};

#[derive(Debug, Clone)]
pub(crate) struct SftpRead {
    sftp: SftpActions,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct ReadParams {
    pub path: RemotePath,

    #[serde(default)]
    pub offset: u64,

    #[serde(default)]
    pub length: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReadResult {
    pub path: String,
    pub offset: u64,
    pub bytes: u64,
    pub eof: bool,
    pub data: String,
}

impl SftpRead {
    pub(super) fn new(sftp: SftpActions) -> Self {
        Self { sftp }
    }
}

impl Action for SftpRead {
    const ID: &'static str = "sftp.read";
    const TITLE: &'static str = "Read part of a file on a host";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = ReadParams;
    type Output = ReadResult;

    fn risk(&self, target: Option<&ResourceRef>, _params: &ReadParams) -> Risk {
        self.sftp.per_call_risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &ReadParams) -> Option<String> {
        self.sftp.destination(target)
    }

    fn summary(&self, output: &ReadResult) -> Value {
        json!({
            "path": output.path,
            "offset": output.offset,
            "bytes": output.bytes,
            "eof": output.eof,
        })
    }

    async fn run(&self, context: &Context, params: ReadParams) -> Result<ReadResult, ActionError> {
        let length = length(params.length)?;

        let (path, offset) = (params.path.as_str(), params.offset);

        let data = self
            .sftp
            .run(context, move |sftp| async move {
                sftp.read(path, offset, length).await
            })
            .await?;
        let bytes = u64::try_from(data.len()).unwrap_or(u64::MAX);

        Ok(ReadResult {
            path: params.path.into_string(),
            offset: params.offset,
            bytes,
            eof: bytes < length,
            data: BASE64.encode(&data),
        })
    }
}

fn length(requested: Option<u64>) -> Result<u64, ActionError> {
    let Some(length) = requested else {
        return Ok(LARGEST_READ);
    };

    if length == 0 || length > LARGEST_READ {
        return Err(ActionError::InvalidParams(format!(
            "length must be between 1 and {LARGEST_READ}, got {length}"
        )));
    }

    Ok(length)
}
