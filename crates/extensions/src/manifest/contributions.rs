mod command;
mod document_type;
mod panel;
mod resource;
mod settings;
mod sidebar_section;
mod status_item;

use std::collections::BTreeSet;
use std::fmt::Display;

use serde::{Deserialize, Serialize};

pub use command::Command;
pub use document_type::{DocumentType, Suffix};
pub use panel::Panel;
pub use resource::{MenuItem, ResourceContributions};
pub use settings::{Settings, SettingsSchema};
pub use sidebar_section::SidebarSection;
pub use status_item::StatusItem;

use crate::ExtensionError;
use crate::manifest::{ActionName, LocalId};

const MOST_ENTRIES: usize = 32;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contributions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sidebar: Vec<SidebarSection>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    document: Vec<DocumentType>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    panel: Vec<Panel>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    command: Vec<Command>,

    #[serde(default, skip_serializing_if = "ResourceContributions::is_empty")]
    resource: ResourceContributions,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    status: Vec<StatusItem>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    settings: Option<Settings>,
}

impl Contributions {
    pub(crate) fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub(crate) fn views(&self) -> BTreeSet<&str> {
        let sidebar = self.sidebar.iter().map(SidebarSection::view);
        let status = self.status.iter().map(StatusItem::view);
        let panels = self.panel.iter().filter_map(Panel::view);

        sidebar
            .chain(status)
            .chain(panels)
            .map(LocalId::as_str)
            .collect()
    }

    pub(crate) fn has_view(&self, view: &str) -> bool {
        self.views().contains(view)
    }

    pub(crate) fn web_panel(&self, id: &str) -> Option<&Panel> {
        self.panel(id).filter(|panel| panel.is_web())
    }

    pub(crate) fn document_type(&self, suffix: &str) -> Option<&DocumentType> {
        self.document
            .iter()
            .find(|kind| kind.suffix().as_str() == suffix)
    }

    pub(crate) fn has_documents(&self) -> bool {
        !self.document.is_empty()
    }

    pub(crate) fn settings(&self) -> Option<&Settings> {
        self.settings.as_ref()
    }

    pub(crate) fn actions(&self) -> impl Iterator<Item = &str> {
        let commands = self.command.iter().map(Command::action);
        let menus = self.resource.menu().iter().map(MenuItem::action);

        commands.chain(menus).map(ActionName::as_str)
    }

    pub(crate) fn check(&self) -> Result<(), ExtensionError> {
        self.check_sizes()?;
        self.check_unique()?;
        self.check_document_panels()
    }

    fn panel(&self, id: &str) -> Option<&Panel> {
        self.panel.iter().find(|panel| panel.id().as_str() == id)
    }

    fn check_sizes(&self) -> Result<(), ExtensionError> {
        let sizes = [
            ("sidebar", self.sidebar.len()),
            ("document", self.document.len()),
            ("panel", self.panel.len()),
            ("command", self.command.len()),
            ("resource.menu", self.resource.menu().len()),
            ("status", self.status.len()),
        ];

        match sizes.into_iter().find(|(_, size)| *size > MOST_ENTRIES) {
            Some((point, size)) => Err(ExtensionError::TooManyEntries {
                point,
                size,
                most: MOST_ENTRIES,
            }),
            None => Ok(()),
        }
    }

    fn check_unique(&self) -> Result<(), ExtensionError> {
        Self::unique("sidebar id", self.sidebar.iter().map(SidebarSection::id))?;
        Self::unique("panel id", self.panel.iter().map(Panel::id))?;
        Self::unique("status id", self.status.iter().map(StatusItem::id))?;
        Self::unique(
            "document type",
            self.document.iter().map(DocumentType::kind),
        )?;
        Self::unique(
            "document suffix",
            self.document.iter().map(DocumentType::suffix),
        )
    }

    fn check_document_panels(&self) -> Result<(), ExtensionError> {
        let missing = self
            .document
            .iter()
            .filter_map(DocumentType::panel)
            .find(|panel| self.panel(panel.as_str()).is_none());

        match missing {
            Some(panel) => Err(ExtensionError::MissingPanel(panel.clone())),
            None => Ok(()),
        }
    }

    fn unique<'a, T: Ord + Display + 'a>(
        field: &'static str,
        values: impl Iterator<Item = &'a T>,
    ) -> Result<(), ExtensionError> {
        let mut seen = BTreeSet::new();

        for value in values {
            if !seen.insert(value) {
                return Err(ExtensionError::Duplicate {
                    field,
                    value: value.to_string(),
                });
            }
        }

        Ok(())
    }
}
