mod fields;
mod kind;

use serde::{Deserialize, Serialize};

use fields::PanelFields;
use kind::PanelKind;
use synt_proto::OneLine;

use crate::ManifestError;
use crate::manifest::{AssetPath, LocalId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PanelFields")]
pub struct Panel {
    id: LocalId,
    title: OneLine,

    #[serde(flatten)]
    kind: PanelKind,
}

impl Panel {
    pub(crate) fn id(&self) -> &LocalId {
        &self.id
    }

    pub(crate) fn title(&self) -> &OneLine {
        &self.title
    }

    pub(crate) fn view(&self) -> Option<&LocalId> {
        match &self.kind {
            PanelKind::View(view) => Some(view),
            PanelKind::Entry(_) => None,
        }
    }

    pub(crate) fn entry(&self) -> Option<&AssetPath> {
        match &self.kind {
            PanelKind::Entry(entry) => Some(entry),
            PanelKind::View(_) => None,
        }
    }

    pub(crate) fn is_web(&self) -> bool {
        self.entry().is_some()
    }
}

impl TryFrom<PanelFields> for Panel {
    type Error = ManifestError;

    fn try_from(fields: PanelFields) -> Result<Self, ManifestError> {
        let PanelFields {
            id,
            title,
            view,
            entry,
        } = fields;

        let kind = match (view, entry) {
            (Some(view), None) => PanelKind::View(view),
            (None, Some(entry)) => PanelKind::Entry(entry),
            _ => return Err(ManifestError::PanelKind(id)),
        };

        Ok(Self { id, title, kind })
    }
}
