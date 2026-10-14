use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub call: Vec<String>,

    #[serde(default)]
    pub storage: bool,
}

impl Capabilities {
    pub fn may_call(&self, action: &str) -> bool {
        self.call.iter().any(|allowed| allowed == action)
    }
}
