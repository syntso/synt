use std::sync::Arc;

use serde_json::Value;
use synt_proto::{PanelPosted, ViewPatched};

use crate::call::FailureCode;
use crate::extensions::Guest;
use crate::interface::GuestError;
use crate::ui::ScreenUpdate;
use crate::view::ViewPatch;

const LARGEST_MESSAGE: usize = 256 * 1024;

pub(super) struct ScreenAccess {
    guest: Arc<Guest>,
}

impl ScreenAccess {
    pub(super) fn new(guest: Arc<Guest>) -> Self {
        Self { guest }
    }

    pub(super) fn patch_view(&self, view: &str, patch: &str) -> Result<(), GuestError> {
        if !self.guest.manifest().contributes().has_view(view) {
            return Err(GuestError::new(
                FailureCode::UnknownView,
                format!("{} contributes no view {view}", self.guest.id()),
            ));
        }

        let patch = Self::parse_message(patch)?;
        let owns = |action: &str| self.guest.owns_action(action);
        ViewPatch::parse(&patch)
            .and_then(|parsed| parsed.check(&owns))
            .map_err(|reason| GuestError::new(FailureCode::InvalidPatch, reason.to_string()))?;

        self.show(ScreenUpdate::ViewPatched(ViewPatched {
            extension: self.guest.id().to_owned(),
            view: view.to_owned(),
            patch,
        }));

        Ok(())
    }

    pub(super) fn post(&self, panel: &str, message: &str) -> Result<(), GuestError> {
        if self
            .guest
            .manifest()
            .contributes()
            .web_panel(panel)
            .is_none()
        {
            return Err(GuestError::new(
                FailureCode::UnknownPanel,
                format!("{} contributes no web panel {panel}", self.guest.id()),
            ));
        }

        let message = Self::parse_message(message)?;

        self.show(ScreenUpdate::PanelPosted(PanelPosted {
            extension: self.guest.id().to_owned(),
            panel: panel.to_owned(),
            message,
        }));

        Ok(())
    }

    fn show(&self, update: ScreenUpdate) {
        if let Some(link) = self.guest.services().link() {
            link.show(update);
        }
    }

    fn parse_message(text: &str) -> Result<Value, GuestError> {
        if text.len() > LARGEST_MESSAGE {
            return Err(GuestError::new(
                FailureCode::TooLarge,
                format!("a message to the app holds at most {LARGEST_MESSAGE} bytes"),
            ));
        }

        serde_json::from_str(text).map_err(|error| {
            GuestError::new(FailureCode::InvalidMessage, format!("not JSON: {error}"))
        })
    }
}
