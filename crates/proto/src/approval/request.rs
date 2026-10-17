use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ApprovalId;
use crate::{ActionId, Actor, Notice, ResourceRef, Risk};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: ApprovalId,
    pub actor: Actor,
    pub action: ActionId,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf_of: Option<Actor>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ResourceRef>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,

    pub risk: Risk,

    #[serde(default)]
    pub params: Value,
}

impl Notice for ApprovalRequest {
    const METHOD: &'static str = "approval.requested";
}
