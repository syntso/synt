use serde::Deserialize;
use synt_proto::OneLine;

use crate::manifest::{AssetPath, LocalId};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PanelFields {
    pub(super) id: LocalId,
    pub(super) title: OneLine,

    #[serde(default)]
    pub(super) view: Option<LocalId>,

    #[serde(default)]
    pub(super) entry: Option<AssetPath>,
}
