use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ActionId, Risk};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionDescription {
    pub id: ActionId,
    pub title: String,
    pub risk: Risk,
    pub person_only: bool,
    pub takes_target: bool,
    pub params: Value,
}
