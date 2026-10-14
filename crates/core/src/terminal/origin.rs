use synt_actions::Risk;
use synt_journal::{Actor, ResourceRef, Seq};

#[derive(Debug, Clone)]
pub(crate) struct Origin {
    pub(crate) action: &'static str,
    pub(crate) actor: Actor,
    pub(crate) target: Option<ResourceRef>,
    pub(crate) request: Seq,
    pub(crate) risk: Risk,
}
