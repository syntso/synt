use synt_journal::{Actor, Seq};

#[derive(Debug, Clone, Copy)]
pub(crate) struct OnBehalf<'a> {
    pub(crate) actor: &'a Actor,
    pub(crate) request: Seq,
}
