use synt_proto::{Notice, Notification, PanelPosted, ViewPatched};

#[derive(Debug, Clone, PartialEq)]
pub enum ScreenUpdate {
    ViewPatched(ViewPatched),
    PanelPosted(PanelPosted),
}

impl ScreenUpdate {
    pub fn to_notification(&self) -> Result<Notification, serde_json::Error> {
        match self {
            Self::ViewPatched(patched) => patched.to_notification(),
            Self::PanelPosted(posted) => posted.to_notification(),
        }
    }
}
