use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::RiskLevel;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionDescription {
    pub id: String,
    pub title: String,
    pub risk: RiskLevel,
    pub person_only: bool,
    pub takes_target: bool,
    pub params: Value,
}
