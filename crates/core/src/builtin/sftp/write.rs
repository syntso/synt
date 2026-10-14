use data_encoding::BASE64;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::WriteMode;

use super::files::LARGEST_WRITE;
use super::{Files, RemotePath};

const DATA_FIELD: &str = "data";
const BASE64_GROUP: usize = 4;
const BYTES_PER_GROUP: usize = 3;

#[derive(Debug, Clone)]
pub(crate) struct SftpWrite {
    files: Files,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct WriteParams {
    pub path: RemotePath,

    #[serde(default)]
    pub offset: u64,

    pub data: String,

    #[serde(default)]
    pub create: bool,

    #[serde(default)]
    pub truncate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct WriteResult {
    pub path: String,
    pub offset: u64,
    pub bytes: u64,
}

impl SftpWrite {
    pub(super) fn new(files: Files) -> Self {
        Self { files }
    }
}

impl Action for SftpWrite {
    const ID: &'static str = "sftp.write";
    const TITLE: &'static str = "Write part of a file on a host";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = WriteParams;
    type Output = WriteResult;

    fn risk(&self, target: Option<&ResourceRef>, _params: &WriteParams) -> Risk {
        self.files.per_call_risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &WriteParams) -> Option<String> {
        self.files.destination(target)
    }

    fn summary(&self, output: &WriteResult) -> Value {
        json!({ "path": output.path, "offset": output.offset, "bytes": output.bytes })
    }

    fn recorded_params(&self, params: &Value) -> Value {
        let Some(fields) = params.as_object() else {
            return params.clone();
        };

        let mut recorded: Map<String, Value> = fields
            .iter()
            .filter(|(name, _)| name.as_str() != DATA_FIELD)
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();

        if let Some(data) = fields.get(DATA_FIELD) {
            let bytes = data.as_str().map(decoded_length);
            recorded.insert("data_bytes".to_owned(), json!(bytes));
        }

        Value::Object(recorded)
    }

    async fn run(
        &self,
        context: &Context,
        params: WriteParams,
    ) -> Result<WriteResult, ActionError> {
        let data = decode(&params.data)?;

        let mode = WriteMode {
            create: params.create,
            truncate: params.truncate,
        };

        let (path, offset, data) = (params.path.as_str(), params.offset, data.as_slice());

        self.files
            .run(context, move |sftp| async move {
                sftp.write(path, offset, data, mode).await
            })
            .await?;

        Ok(WriteResult {
            path: params.path.into_string(),
            offset: params.offset,
            bytes: u64::try_from(data.len()).unwrap_or(u64::MAX),
        })
    }
}

fn decode(encoded: &str) -> Result<Vec<u8>, ActionError> {
    let length = decoded_length(encoded);

    if u64::try_from(length).unwrap_or(u64::MAX) > LARGEST_WRITE {
        return Err(ActionError::InvalidParams(format!(
            "data holds {length} bytes; at most {LARGEST_WRITE} may be written in one call"
        )));
    }

    BASE64
        .decode(encoded.as_bytes())
        .map_err(|error| ActionError::InvalidParams(format!("data is not base64: {error}")))
}

fn decoded_length(encoded: &str) -> usize {
    let padding = encoded
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'=')
        .count();
    let groups = encoded.len().div_ceil(BASE64_GROUP);

    (groups * BYTES_PER_GROUP).saturating_sub(padding)
}
