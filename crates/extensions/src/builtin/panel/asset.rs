mod file;

use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};

use file::AssetFile;

use super::mime::mime_of;
use crate::Extensions;
use crate::blocking::Blocking;
use crate::builtin::CallingActor;
use crate::manifest::AssetPath;

const LARGEST_ASSET: usize = 8 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct ExtensionPanelAsset {
    extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AssetParams {
    extension: String,
    path: String,
}

impl ExtensionPanelAsset {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl AssetParams {
    fn checked(&self) -> Result<(AssetPath, &'static str), ActionError> {
        let refused = || {
            ActionError::InvalidParams(format!(
                "{:?} is not a file a web panel may load",
                self.path
            ))
        };

        let path = AssetPath::try_from(self.path.clone()).map_err(|_| refused())?;
        let mime = mime_of(path.as_str()).ok_or_else(refused)?;

        Ok((path, mime))
    }
}

impl Action for ExtensionPanelAsset {
    const ID: &'static str = "extension.panel.asset";
    const TITLE: &'static str = "Read one file of an extension's web panel bundle";
    const RISK: Risk = Risk::Read;

    type Params = AssetParams;
    type Output = AssetFile;

    fn summary(&self, output: &AssetFile) -> Value {
        output.summary()
    }

    async fn run(&self, context: &Context, params: AssetParams) -> Result<AssetFile, ActionError> {
        context.refuse_extensions("reach the views, panels or actions of an extension")?;

        let (path, mime) = params.checked()?;
        let guest = Arc::clone(self.extensions.loaded(&params.extension)?.guest());
        let wanted = path.as_str().to_owned();

        let found = Blocking::run(move || guest.asset(&wanted)).await?;
        let data = found
            .filter(|data| data.len() <= LARGEST_ASSET)
            .ok_or_else(|| {
                ActionError::failed(format!(
                    "{} has no panel file {path} of at most {LARGEST_ASSET} bytes",
                    params.extension
                ))
            })?;

        Ok(AssetFile::new(path, mime, &data))
    }
}
