use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Notice;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelPosted {
    pub extension: String,
    pub panel: String,
    pub message: Value,
}

impl Notice for PanelPosted {
    const METHOD: &'static str = "extension.panel.posted";
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::PanelPosted;
    use crate::Notice;

    #[test]
    fn a_panel_message_travels_as_a_notification() {
        let posted = PanelPosted {
            extension: "uml".to_owned(),
            panel: "editor".to_owned(),
            message: json!({ "saved": "orders.uml" }),
        };

        let notification = posted.to_notification().unwrap();

        assert_eq!(notification.method, "extension.panel.posted");
        assert_eq!(notification.params::<PanelPosted>().unwrap(), posted);
    }
}
