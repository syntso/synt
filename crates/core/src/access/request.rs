use serde_json::Value;
use synt_actions::Risk;
use synt_journal::{ActionId, Actor, ResourceRef};
use tokio_util::sync::CancellationToken;

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
