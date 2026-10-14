use data_encoding::HEXLOWER;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_proto::StreamId;
use synt_ssh::Download;

use super::files::{failed, link};
use super::{Files, RemotePath};
use crate::link::Outbound;

#[derive(Debug, Clone)]
pub(crate) struct SftpDownload {
    files: Files,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct DownloadParams {
    pub path: RemotePath,
    #[schemars(with = "u32")]
    pub stream: StreamId,

    #[serde(default)]
    pub offset: u64,

    #[serde(default)]
    pub length: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Downloaded {
    pub path: String,
    pub offset: u64,
    pub bytes: u64,
    pub eof: bool,
    pub sha256: String,
}

struct Sent {
    bytes: u64,
    sha256: String,
}

impl SftpDownload {
    pub(super) fn new(files: Files) -> Self {
        Self { files }
    }

    async fn open(
        &self,
        context: &Context,
        params: &DownloadParams,
    ) -> Result<Download, ActionError> {
        let (path, offset, length) = (params.path.as_str(), params.offset, params.length);

        self.files
            .run(context, move |sftp| async move {
                sftp.download(path, offset, length).await
            })
            .await
    }

    async fn transfer(
        &self,
        context: &Context,
        params: &DownloadParams,
        outbound: &mut Outbound,
    ) -> Result<Sent, ActionError> {
        let mut download = self.open(context, params).await?;
        let mut hasher = Sha256::new();
        let mut bytes = 0;

        while let Some(data) = download.next().await.map_err(|error| failed(&error))? {
            hasher.update(&data);
            bytes += u64::try_from(data.len()).unwrap_or(u64::MAX);

            outbound.send(data).await?;
        }

        Ok(Sent {
            bytes,
            sha256: HEXLOWER.encode(&hasher.finalize()),
        })
    }
}

impl Action for SftpDownload {
    const ID: &'static str = "sftp.download";
    const TITLE: &'static str = "Download a file from a host";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = DownloadParams;
    type Output = Downloaded;

    fn risk(&self, target: Option<&ResourceRef>, _params: &DownloadParams) -> Risk {
        self.files.per_call_risk(target, Self::RISK)
    }

    fn destination(
        &self,
        target: Option<&ResourceRef>,
        _params: &DownloadParams,
    ) -> Option<String> {
        self.files.destination(target)
    }

    fn summary(&self, output: &Downloaded) -> Value {
        json!({
            "path": output.path,
            "offset": output.offset,
            "bytes": output.bytes,
            "eof": output.eof,
            "sha256": output.sha256,
        })
    }

    async fn run(
        &self,
        context: &Context,
        params: DownloadParams,
    ) -> Result<Downloaded, ActionError> {
        if params.length == Some(0) {
            return Err(ActionError::InvalidParams(
                "length must be at least 1 when given".to_owned(),
            ));
        }

        let _slot = self.files.transfer_slot().await?;
        let link = link(context)?;

        let mut outbound = Outbound::open(&link, params.stream)?;
        let sent = self.transfer(context, &params, &mut outbound).await;
        outbound.finish().await;
        let sent = sent?;

        Ok(Downloaded {
            path: params.path.into_string(),
            offset: params.offset,
            bytes: sent.bytes,
            eof: params.length.is_none_or(|length| sent.bytes < length),
            sha256: sent.sha256,
        })
    }
}
