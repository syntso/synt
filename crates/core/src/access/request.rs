use serde_json::Value;
use synt_actions::Risk;
use synt_journal::{ActionId, Actor, ResourceRef};
use synt_proto::{ApprovalId, ApprovalRequest};
use tokio_util::sync::CancellationToken;

use crate::broker::{requester, risk_level};

#[derive(Debug, Clone, Copy)]
pub struct AccessRequest<'a> {
    pub actor: &'a Actor,
    pub origin: Option<&'a Actor>,
    pub action: &'a ActionId,
    pub risk: Risk,
    pub target: Option<&'a ResourceRef>,
    pub destination: Option<&'a str>,
    pub grantable: bool,
    pub params: &'a Value,
    pub run_approved: bool,
    pub departed: &'a CancellationToken,
}

impl AccessRequest<'_> {
    pub(crate) fn approval(&self, id: ApprovalId) -> ApprovalRequest {
        ApprovalRequest {
            id,
            actor: requester(self.actor),
            on_behalf_of: self.origin.map(requester),
            action: self.action.as_str().to_owned(),
            target: self.target.map(|target| target.as_str().to_owned()),
            destination: self.destination.map(str::to_owned),
            risk: risk_level(self.risk),
            params: self.params.clone(),
        }
    }
}
