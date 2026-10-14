use serde_json::Value;

use super::Visit;
use crate::interface::GuestError;
use crate::manifest::Panel;
use crate::screen::ScreenUpdate;
use crate::view::ViewPatch;

const LARGEST_MESSAGE: usize = 256 * 1024;

impl Visit {
    pub(super) fn send_patch(&self, view: &str, patch: &str) -> Result<(), GuestError> {
        if !self.manifest().contributes.views().contains(view) {
            return Err(GuestError::new(
                "unknown_view",
                format!("{} contributes no view {view}", self.id()),
            ));
        }

        let patch = parse_message(patch)?;
        let owns = |action: &str| self.guest.owns_action(action);
        ViewPatch::check(&patch, &owns)
            .map_err(|reason| GuestError::new("invalid_patch", reason))?;

        self.show(ScreenUpdate::ViewPatched {
            extension: self.id().to_owned(),
            view: view.to_owned(),
            patch,
        });

        Ok(())
    }

    pub(super) fn send_message(&self, panel: &str, message: &str) -> Result<(), GuestError> {
        let is_web_panel = self
            .manifest()
            .contributes
            .panel(panel)
            .is_some_and(Panel::is_web);

        if !is_web_panel {
            return Err(GuestError::new(
                "unknown_panel",
                format!("{} contributes no web panel {panel}", self.id()),
            ));
        }

        let message = parse_message(message)?;

        self.show(ScreenUpdate::PanelPosted {
            extension: self.id().to_owned(),
            panel: panel.to_owned(),
            message,
        });

        Ok(())
    }

    fn show(&self, update: ScreenUpdate) {
        if let Some(screen) = self.guest.services().screen.get() {
            screen.show(update);
        }
    }
}

fn parse_message(text: &str) -> Result<Value, GuestError> {
    if text.len() > LARGEST_MESSAGE {
        return Err(GuestError::new(
            "too_large",
            format!("a message to the app holds at most {LARGEST_MESSAGE} bytes"),
        ));
    }

    serde_json::from_str(text)
        .map_err(|error| GuestError::new("invalid_message", format!("not JSON: {error}")))
}
