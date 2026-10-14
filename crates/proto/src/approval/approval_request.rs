use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ApprovalId, Requester, RiskLevel};
use crate::Notification;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: ApprovalId,
    pub actor: Requester,
    pub action: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf_of: Option<Requester>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,

    pub risk: RiskLevel,

    #[serde(default)]
    pub params: Value,
}

impl ApprovalRequest {
    pub const METHOD: &'static str = "approval.requested";

    pub fn to_notification(&self) -> Result<Notification, serde_json::Error> {
        let params = serde_json::to_value(self)?;
        Ok(Notification::new(Self::METHOD, params))
    }
}
