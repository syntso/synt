use synt_actions::{ActionError, Registry};

use super::contributions::ExtensionContributions;
use super::document::ExtensionDocuments;
use super::file::{ExtensionFileForget, ExtensionFileList, ExtensionFilePick};
use super::http::ExtensionHttp;
use super::lifecycle::{
    ExtensionDisable, ExtensionEnable, ExtensionInstall, ExtensionInstallLocal, ExtensionList,
    ExtensionRemove, ExtensionSearch,
};
use super::panel::{ExtensionPanelAsset, ExtensionPanelMessage, ExtensionPanelOpen};
use super::settings::{ExtensionSettingsGet, ExtensionSettingsSet};
use super::view::ExtensionViewRender;
use crate::Extensions;

pub(crate) fn register(
    extensions: &Extensions,
    registry: &mut Registry,
) -> Result<(), ActionError> {
    register_lifecycle(extensions, registry)?;
    register_surfaces(extensions, registry)?;
    register_reach(extensions, registry)
}

fn register_lifecycle(extensions: &Extensions, registry: &mut Registry) -> Result<(), ActionError> {
    registry.register(ExtensionSearch::new(extensions.clone()))?;
    registry.register(ExtensionInstall::new(extensions.clone()))?;
    registry.register(ExtensionInstallLocal::new(extensions.clone()))?;
    registry.register(ExtensionList::new(extensions.clone()))?;
    registry.register(ExtensionEnable::new(extensions.clone()))?;
    registry.register(ExtensionDisable::new(extensions.clone()))?;
    registry.register(ExtensionRemove::new(extensions.clone()))
}

fn register_surfaces(extensions: &Extensions, registry: &mut Registry) -> Result<(), ActionError> {
    registry.register(ExtensionContributions::new(extensions.clone()))?;
    registry.register(ExtensionViewRender::new(extensions.clone()))?;
    registry.register(ExtensionPanelOpen::new(extensions.clone()))?;
    registry.register(ExtensionPanelAsset::new(extensions.clone()))?;
    registry.register(ExtensionPanelMessage::new(extensions.clone()))?;
    registry.register(ExtensionSettingsGet::new(extensions.clone()))?;
    registry.register(ExtensionSettingsSet::new(extensions.clone()))
}

fn register_reach(extensions: &Extensions, registry: &mut Registry) -> Result<(), ActionError> {
    registry.register(ExtensionHttp::new(extensions.clone()))?;
    registry.register(ExtensionDocuments::new(extensions.clone()))?;
    registry.register(ExtensionFilePick::new(extensions.clone()))?;
    registry.register(ExtensionFileForget::new(extensions.clone()))?;
    registry.register(ExtensionFileList::new(extensions.clone()))
}
