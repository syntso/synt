use std::sync::Arc;

use data_encoding::BASE64;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::asset_file::Asset;
use super::asset_params::AssetParams;
use super::mime::mime_of;
use crate::Extensions;
use crate::builtin::guard::refuse_extensions;
use crate::builtin::guest_thread::on_guest_thread;
use crate::manifest::is_asset_path;

const LARGEST_ASSET: usize = 8 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct ExtensionPanelAsset {
    extensions: Extensions,
}

impl ExtensionPanelAsset {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionPanelAsset {
    const ID: &'static str = "extension.panel.asset";
    const TITLE: &'static str = "Read one file of an extension's web panel bundle";
    const RISK: Risk = Risk::Read;

    type Params = AssetParams;
    type Output = Asset;

    fn summary(&self, output: &Asset) -> Value {
        json!({ "path": output.path, "mime": output.mime, "bytes": output.bytes })
    }

    async fn run(&self, context: &Context, params: AssetParams) -> Result<Asset, ActionError> {
        refuse_extensions(context)?;

        let mime = mime_of(&params.path)
            .filter(|_| is_asset_path(&params.path))
            .ok_or_else(|| {
                ActionError::InvalidParams(format!(
                    "{:?} is not a file a web panel may load",
                    params.path
                ))
            })?;

        let loaded = self
            .extensions
            .loaded(&params.extension)
            .map_err(|error| ActionError::failed(error.to_string()))?;
        let guest = Arc::clone(&loaded.guest);
        let path = params.path.clone();

        let found = on_guest_thread(move |_| guest.asset(&path).map(Ok)).await?;
        let data = found
            .filter(|data| data.len() <= LARGEST_ASSET)
            .ok_or_else(|| {
                ActionError::failed(format!(
                    "{} has no panel file {} of at most {LARGEST_ASSET} bytes",
                    params.extension, params.path
                ))
            })?;

        Ok(Asset {
            path: params.path,
            mime,
            bytes: data.len(),
            data: BASE64.encode(&data),
        })
    }
}
