use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::asset_file::AssetFile;
use super::mime::mime_of;
use crate::Extensions;
use crate::builtin::guard::refuse_extensions;
use crate::extension::on_guest_thread;
use crate::manifest::is_asset_path;

const LARGEST_ASSET: usize = 8 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct ExtensionPanelAsset {
    extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AssetParams {
    pub extension: String,
    pub path: String,
}

impl ExtensionPanelAsset {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl AssetParams {
    fn mime(&self) -> Result<&'static str, ActionError> {
        mime_of(&self.path)
            .filter(|_| is_asset_path(&self.path))
            .ok_or_else(|| {
                ActionError::InvalidParams(format!(
                    "{:?} is not a file a web panel may load",
                    self.path
                ))
            })
    }
}

impl Action for ExtensionPanelAsset {
    const ID: &'static str = "extension.panel.asset";
    const TITLE: &'static str = "Read one file of an extension's web panel bundle";
    const RISK: Risk = Risk::Read;

    type Params = AssetParams;
    type Output = AssetFile;

    fn summary(&self, output: &AssetFile) -> Value {
        json!({ "path": output.path, "mime": output.mime, "bytes": output.bytes })
    }

    async fn run(&self, context: &Context, params: AssetParams) -> Result<AssetFile, ActionError> {
        refuse_extensions(context)?;

        let mime = params.mime()?;
        let guest = Arc::clone(&self.extensions.loaded(&params.extension)?.guest);
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

        Ok(AssetFile::new(params.path, mime, &data))
    }
}
