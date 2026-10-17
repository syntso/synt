use data_encoding::HEXLOWER;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_proto::StreamId;
use synt_ssh::{Upload, WriteMode};

use super::{RemotePath, SftpActions};
use crate::CoreError;
use crate::link::Inbound;
use crate::link::Link;

#[derive(Debug, Clone)]
pub(crate) struct SftpUpload {
    sftp: SftpActions,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct UploadParams {
    pub path: RemotePath,
    #[schemars(with = "u32")]
    pub stream: StreamId,
    pub size: u64,

    #[serde(default)]
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Uploaded {
    pub path: String,
    pub offset: u64,
    pub bytes: u64,
    pub sha256: String,
}

impl SftpUpload {
    pub(super) fn new(sftp: SftpActions) -> Self {
        Self { sftp }
    }

    async fn open(&self, context: &Context, params: &UploadParams) -> Result<Upload, ActionError> {
        let (path, offset) = (params.path.as_str(), params.offset);

        if offset > 0 {
            self.check_resume(context, path, offset).await?;
        }

        let mode = WriteMode {
            create: true,
            truncate: offset == 0,
        };

        self.sftp
            .run(context, move |sftp| async move {
                sftp.upload(path, offset, mode, true).await
            })
            .await
    }

    async fn check_resume(
        &self,
        context: &Context,
        path: &str,
        offset: u64,
    ) -> Result<(), ActionError> {
        let entry = self
            .sftp
            .run(context, move |sftp| async move { sftp.stat(path).await })
            .await?;

        if entry.size == Some(offset) {
            return Ok(());
        }

        let held = entry.size.map_or_else(
            || "an unknown number of".to_owned(),
            |size| size.to_string(),
        );

        Err(ActionError::InvalidParams(format!(
            "{path} holds {held} bytes, so an upload cannot resume at {offset}"
        )))
    }
}

impl Action for SftpUpload {
    const ID: &'static str = "sftp.upload";
    const TITLE: &'static str = "Upload a file to a host";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = UploadParams;
    type Output = Uploaded;

    fn risk(&self, target: Option<&ResourceRef>, _params: &UploadParams) -> Risk {
        self.sftp.per_call_risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &UploadParams) -> Option<String> {
        self.sftp.destination(target)
    }

    fn summary(&self, output: &Uploaded) -> Value {
        json!({
            "path": output.path,
            "offset": output.offset,
            "bytes": output.bytes,
            "sha256": output.sha256,
        })
    }

    async fn run(&self, context: &Context, params: UploadParams) -> Result<Uploaded, ActionError> {
        let _slot = self.sftp.transfer_slot().await?;
        let link = Link::of_call(context, "the file")?;

        let mut inbound = Inbound::open(&link, params.stream).await?;
        let mut upload = self.open(context, &params).await?;

        let sha256 = receive(&mut inbound, &mut upload, params.size).await?;
        upload.finish().await.map_err(CoreError::from)?;

        Ok(Uploaded {
            path: params.path.into_string(),
            offset: params.offset,
            bytes: params.size,
            sha256,
        })
    }
}

async fn receive(
    inbound: &mut Inbound,
    upload: &mut Upload,
    size: u64,
) -> Result<String, ActionError> {
    let mut hasher = Sha256::new();
    let mut received = 0;

    while received < size {
        let Some(data) = inbound.next().await? else {
            return Err(ActionError::failed(format!(
                "the client stopped after {received} of {size} bytes"
            )));
        };

        received += u64::try_from(data.len()).unwrap_or(u64::MAX);

        if received > size {
            return Err(ActionError::failed(format!(
                "the client sent more than the {size} bytes it announced"
            )));
        }

        hasher.update(&data);
        upload.write(&data).await.map_err(CoreError::from)?;
    }

    Ok(HEXLOWER.encode(&hasher.finalize()))
}
