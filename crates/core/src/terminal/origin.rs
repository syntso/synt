use synt_actions::Risk;
use synt_journal::{Actor, ResourceRef, Seq};

#[derive(Debug, Clone)]
pub(crate) struct Origin {
    pub(crate) actor: Actor,
    pub(crate) target: ResourceRef,
    pub(crate) request: Seq,
    pub(crate) risk: Risk,
}
