use serde_json::Value;
use synt_actions::Risk;
use synt_journal::{ActionId, Actor, ResourceRef};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy)]
pub struct AccessRequest<'a> {
    pub actor: &'a Actor,
    pub action: &'a ActionId,
    pub risk: Risk,
    pub target: Option<&'a ResourceRef>,
    pub params: &'a Value,
    pub departed: &'a CancellationToken,
}
