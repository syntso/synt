use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{
    Command, DocumentType, MenuItem, Panel, ResourceContributions, Settings, SidebarSection,
    StatusItem,
};
use crate::ExtensionError;
use crate::manifest::checks::refused;

const MOST_ENTRIES: usize = 32;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contributions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidebar: Vec<SidebarSection>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub document: Vec<DocumentType>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub panel: Vec<Panel>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<Command>,

    #[serde(default, skip_serializing_if = "ResourceContributions::is_empty")]
    pub resource: ResourceContributions,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status: Vec<StatusItem>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>,
}

impl Contributions {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub fn views(&self) -> BTreeSet<&str> {
        let sidebar = self.sidebar.iter().map(|section| section.view.as_str());
        let status = self.status.iter().map(|item| item.view.as_str());
        let panels = self.panel.iter().filter_map(|panel| panel.view.as_deref());

        sidebar.chain(status).chain(panels).collect()
    }

    pub fn panel(&self, id: &str) -> Option<&Panel> {
        self.panel.iter().find(|panel| panel.id == id)
    }

    pub fn document_type(&self, suffix: &str) -> Option<&DocumentType> {
        self.document.iter().find(|kind| kind.suffix == suffix)
    }

    pub fn actions(&self) -> impl Iterator<Item = &str> {
        let commands = self.command.iter().map(|command| command.action.as_str());
        let menus = self.resource.menu.iter().map(|item| item.action.as_str());

        commands.chain(menus)
    }

    pub(crate) fn check(&self) -> Result<(), ExtensionError> {
        self.check_sizes()?;
        self.check_entries()?;
        self.check_unique()?;
        self.check_document_panels()
    }

    fn check_sizes(&self) -> Result<(), ExtensionError> {
        let sizes = [
            ("sidebar", self.sidebar.len()),
            ("document", self.document.len()),
            ("panel", self.panel.len()),
            ("command", self.command.len()),
            ("resource.menu", self.resource.menu.len()),
            ("status", self.status.len()),
        ];

        match sizes.iter().find(|(_, size)| *size > MOST_ENTRIES) {
            Some((point, size)) => Err(refused(format!(
                "contributes.{point} has {size} entries; the most is {MOST_ENTRIES}"
            ))),
            None => Ok(()),
        }
    }

    fn check_entries(&self) -> Result<(), ExtensionError> {
        self.sidebar.iter().try_for_each(SidebarSection::check)?;
        self.document.iter().try_for_each(DocumentType::check)?;
        self.panel.iter().try_for_each(Panel::check)?;
        self.command.iter().try_for_each(Command::check)?;
        self.resource.menu.iter().try_for_each(MenuItem::check)?;
        self.status.iter().try_for_each(StatusItem::check)?;

        self.settings.as_ref().map_or(Ok(()), Settings::check)
    }

    fn check_unique(&self) -> Result<(), ExtensionError> {
        unique("sidebar id", self.sidebar.iter().map(|section| &section.id))?;
        unique("panel id", self.panel.iter().map(|panel| &panel.id))?;
        unique("status id", self.status.iter().map(|item| &item.id))?;
        unique("document type", self.document.iter().map(|kind| &kind.kind))?;
        unique(
            "document suffix",
            self.document.iter().map(|kind| &kind.suffix),
        )
    }

    fn check_document_panels(&self) -> Result<(), ExtensionError> {
        let missing = self
            .document
            .iter()
            .filter_map(|kind| kind.panel.as_ref())
            .find(|panel| self.panel(panel).is_none());

        match missing {
            Some(panel) => Err(refused(format!(
                "a document type opens in panel {panel}, which is not contributed"
            ))),
            None => Ok(()),
        }
    }
}

fn unique<'a>(field: &str, values: impl Iterator<Item = &'a String>) -> Result<(), ExtensionError> {
    let mut seen = BTreeSet::new();

    for value in values {
        if !seen.insert(value) {
            return Err(refused(format!("{field} {value:?} is used twice")));
        }
    }

    Ok(())
}
