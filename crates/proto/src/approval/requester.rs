use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "name", rename_all = "snake_case")]
pub enum Requester {
    Person,
    Core,
    Agent(String),
    Runbook(String),
    Plugin(String),
}
